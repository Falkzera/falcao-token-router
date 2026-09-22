//! A credencial de um perfil — o "chaveiro" do porte (≙ `KeychainStore`).
//!
//! No Windows o Claude Code guarda a credencial num ARQUIVO,
//! `<perfil>\.credentials.json` — nada no Credential Manager (doc oficial e
//! máquina conferidas no spike de 22/09/2026). O "item de chaveiro" do macOS vira
//! um arquivo por perfil, sem hash; a "chave" é o caminho dele, que o adapter
//! deriva do perfil (`ProviderAdapter::credential_location`).
//!
//! Três regras seguram a troca a quente e as invariantes:
//!
//! - **Blob opaco.** O conteúdo é `Vec<u8>` e nunca é decodificado para usar um
//!   token; nenhum tipo aqui tem campo de token. A única leitura é estrutural:
//!   "é um JSON completo com o objeto `claudeAiOauth`?", sem materializar valor.
//! - **Só se propaga blob completo.** O Claude Code grava por staging + rename,
//!   mas tem um braço in-place de reserva; um arquivo pego no meio disso não é
//!   credencial. A leitura espera um pouco e, persistindo, não devolve nada.
//! - **Mtime novo a cada escrita, e nenhuma escrita à toa.** O Claude Code só
//!   relê a credencial quando o mtime muda (JS do binário 2.1.280: `stat` antes
//!   de decidir o refresh). Escrever sai por temp+rename; bytes idênticos não
//!   são regravados.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use serde::de::{Deserialize, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

use super::default_profile_guard::DefaultProfileGuard;
use crate::platform::atomic_write::{read_retrying, remove_retrying, write_atomic};

/// A credencial de um perfil: os bytes EXATOS do `.credentials.json`, opacos.
///
/// Não implementa `Serialize`/`Deserialize` nem expõe campo nenhum, e o `Debug`
/// não mostra o conteúdo — um `{:?}` descuidado num log não vaza nada.
#[derive(Clone, PartialEq, Eq)]
pub struct CredentialBlob(Vec<u8>);

impl CredentialBlob {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        CredentialBlob(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// JSON completo cujo objeto raiz tem `claudeAiOauth` como objeto. Checagem
    /// de FORMA: as chaves são lidas, os valores são pulados sem ser guardados.
    pub fn is_complete(&self) -> bool {
        let mut de = serde_json::Deserializer::from_slice(&self.0);
        matches!(de.deserialize_map(RootShape), Ok(true)) && de.end().is_ok()
    }
}

impl fmt::Debug for CredentialBlob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CredentialBlob(<{} bytes>)", self.0.len())
    }
}

/// Visita o objeto raiz e responde se `claudeAiOauth` é um objeto.
struct RootShape;

impl<'de> Visitor<'de> for RootShape {
    type Value = bool;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("um objeto JSON")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<bool, A::Error> {
        let mut found = false;
        while let Some(key) = map.next_key::<String>()? {
            if key == "claudeAiOauth" {
                found = map.next_value::<IsObject>()?.0;
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        Ok(found)
    }
}

/// Um valor que só diz se é objeto — o conteúdo é pulado, nunca guardado.
struct IsObject(bool);

impl<'de> Deserialize<'de> for IsObject {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(IsObjectVisitor)
    }
}

struct IsObjectVisitor;

impl<'de> Visitor<'de> for IsObjectVisitor {
    type Value = IsObject;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("qualquer valor JSON")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<IsObject, A::Error> {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(IsObject(true))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<IsObject, A::Error> {
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(IsObject(false))
    }

    fn visit_bool<E>(self, _: bool) -> Result<IsObject, E> {
        Ok(IsObject(false))
    }

    fn visit_i64<E>(self, _: i64) -> Result<IsObject, E> {
        Ok(IsObject(false))
    }

    fn visit_u64<E>(self, _: u64) -> Result<IsObject, E> {
        Ok(IsObject(false))
    }

    fn visit_f64<E>(self, _: f64) -> Result<IsObject, E> {
        Ok(IsObject(false))
    }

    fn visit_str<E>(self, _: &str) -> Result<IsObject, E> {
        Ok(IsObject(false))
    }

    fn visit_unit<E>(self) -> Result<IsObject, E> {
        Ok(IsObject(false))
    }
}

/// Por que uma credencial não foi gravada.
#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    /// O blob não tem a forma de uma credencial completa — recusado, para nunca
    /// espalhar um arquivo pego no meio de uma escrita.
    #[error("credencial incompleta (sem o objeto claudeAiOauth) — recusada")]
    Incomplete,
    /// A guarda do perfil padrão não conseguiu guardar o login de antes; sem a
    /// cópia, o perfil padrão não é tocado.
    #[error("não foi possível guardar o login do perfil padrão antes de escrever: {0}")]
    BackupFailed(#[source] io::Error),
    /// Falha de E/S ao gravar.
    #[error("falha ao gravar {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Lê e escreve credenciais de perfil. Injetável: a implementação real é o
/// [`FileCredentialStore`]; os testes do motor passam uma em memória. O motor de
/// rotação só conhece isto.
pub trait CredentialStore: Send + Sync {
    /// O blob do perfil, ou `None` se não existe (ou não é uma credencial completa).
    fn read(&self, location: &Path) -> Option<CredentialBlob>;
    /// Cria ou atualiza.
    fn write(&self, blob: &CredentialBlob, location: &Path) -> Result<(), CredentialError>;
    /// Há uma credencial completa ali?
    fn exists(&self, location: &Path) -> bool;
    /// Apaga. Silencioso se já não existe.
    ///
    /// Existe porque "remover a conta" tem de remover a credencial dela: um app
    /// que tira a conta da tela e deixa o refresh token vivo no disco promete uma
    /// coisa e faz outra.
    fn delete(&self, location: &Path);
}

/// Quantas vezes reler um arquivo incompleto antes de desistir, e o intervalo.
/// ~0,2 s: tempo de sobra para o braço in-place do Claude Code terminar.
const INCOMPLETE_ATTEMPTS: usize = 5;
const INCOMPLETE_WAIT: Duration = Duration::from_millis(40);

/// A credencial como o Claude Code a guarda no Windows: um arquivo por perfil.
#[derive(Default)]
pub struct FileCredentialStore {
    guard: Option<DefaultProfileGuard>,
}

impl FileCredentialStore {
    pub fn new() -> Self {
        FileCredentialStore { guard: None }
    }

    /// Com a guarda do perfil padrão: antes da primeira escrita no `~\.claude`,
    /// o login que estava lá é copiado para `<base>\backups\`.
    pub fn with_guard(guard: DefaultProfileGuard) -> Self {
        FileCredentialStore { guard: Some(guard) }
    }

    fn read_once(location: &Path) -> Option<CredentialBlob> {
        let bytes = read_retrying(location).ok()?;
        let blob = CredentialBlob::from_bytes(bytes);
        blob.is_complete().then_some(blob)
    }
}

impl CredentialStore for FileCredentialStore {
    fn read(&self, location: &Path) -> Option<CredentialBlob> {
        for attempt in 0..INCOMPLETE_ATTEMPTS {
            if attempt > 0 {
                thread::sleep(INCOMPLETE_WAIT);
            }
            match read_retrying(location) {
                Ok(bytes) => {
                    let blob = CredentialBlob::from_bytes(bytes);
                    if blob.is_complete() {
                        return Some(blob);
                    }
                    // Incompleto: talvez uma escrita em andamento. Espera e relê.
                }
                // Ausente (ou ilegível mesmo depois das tentativas): não há o que ler.
                Err(_) => return None,
            }
        }
        None
    }

    fn write(&self, blob: &CredentialBlob, location: &Path) -> Result<(), CredentialError> {
        if !blob.is_complete() {
            return Err(CredentialError::Incomplete);
        }
        // Bytes idênticos: nada a fazer. Um mtime novo sem motivo faria toda
        // sessão viva do perfil descartar o cache e reler à toa.
        if read_retrying(location).is_ok_and(|current| current == blob.as_bytes()) {
            return Ok(());
        }
        if let Some(guard) = &self.guard {
            if guard.protects(location) {
                guard.backup_once().map_err(CredentialError::BackupFailed)?;
            }
        }
        write_atomic(location, blob.as_bytes()).map_err(|source| CredentialError::Io {
            path: location.to_path_buf(),
            source,
        })
    }

    fn exists(&self, location: &Path) -> bool {
        Self::read_once(location).is_some()
    }

    fn delete(&self, location: &Path) {
        let _ = remove_retrying(location);
    }
}
