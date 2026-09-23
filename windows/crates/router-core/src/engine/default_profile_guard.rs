//! A guarda do perfil padrão (`%USERPROFILE%\.claude`) — novo no porte.
//!
//! O grupo padrão roda no `~\.claude`, o mesmo perfil do `claude` puro. Numa
//! máquina onde ele já tem um login que o router não conhece, a primeira
//! ativação sobrescreveria credencial e identidade sem volta. O macOS escreve lá
//! direto; o porte guarda antes uma cópia do que havia — uma vez só, na primeira
//! escrita — em `<base>\backups\default-profile-<carimbo>\`:
//! `.credentials.json` (bytes exatos) e `oauthAccount.json` (a identidade).
//!
//! A confirmação na UI quando o `~\.claude` tem um login desconhecido é da Fase
//! 5; esta cópia é o piso, vale também para a CLI.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde_json::Value;

use super::anthropic_adapter::AnthropicAdapter;
use super::config_dir::ConfigDir;
use super::provider::ProviderAdapter;
use crate::platform::atomic_write::{read_retrying, write_atomic};

/// Prefixo das pastas de cópia — é por ele que se sabe que a cópia já foi feita.
const PREFIX: &str = "default-profile-";

pub struct DefaultProfileGuard {
    credential: PathBuf,
    claude_json: PathBuf,
    backups: PathBuf,
}

impl DefaultProfileGuard {
    /// `credential` e `claude_json` são os do perfil padrão; `base` é a raiz do
    /// router (`<base>\backups\` recebe as cópias).
    pub fn new(credential: PathBuf, claude_json: PathBuf, base: &Path) -> Self {
        DefaultProfileGuard {
            credential,
            claude_json,
            backups: base.join("backups"),
        }
    }

    /// A guarda do `<home>\.claude` do usuário, com as cópias sob `base`.
    pub fn for_home(home: &str, base: &Path) -> Self {
        let dir = ConfigDir::standard(home);
        Self::new(
            AnthropicAdapter.credential_location(&dir),
            dir.global_config_path(),
            base,
        )
    }

    /// Esta escrita é no perfil padrão? Comparação sem caixa e sem ligar para a
    /// barra (`C:/…` e `C:\…` são o mesmo arquivo no Windows).
    pub fn protects(&self, location: &Path) -> bool {
        normalize(location) == normalize(&self.credential)
    }

    /// Já existe uma cópia de antes da primeira escrita?
    pub fn already_backed_up(&self) -> bool {
        fs::read_dir(&self.backups).is_ok_and(|entries| {
            entries
                .filter_map(Result::ok)
                .any(|e| e.file_name().to_string_lossy().starts_with(PREFIX))
        })
    }

    /// Copia o login atual do perfil padrão, se ainda não foi copiado. Devolve a
    /// pasta criada, ou `None` quando não havia nada a guardar (ou já havia cópia).
    pub fn backup_once(&self) -> io::Result<Option<PathBuf>> {
        if self.already_backed_up() {
            return Ok(None);
        }
        let credential = match read_retrying(&self.credential) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        let account = read_retrying(&self.claude_json)
            .ok()
            .and_then(|data| serde_json::from_slice::<Value>(&data).ok())
            .and_then(|root| root.get("oauthAccount").cloned());
        if credential.is_none() && account.is_none() {
            // Perfil padrão sem login: não há o que perder.
            return Ok(None);
        }

        let dir = self
            .backups
            .join(format!("{PREFIX}{}", Utc::now().format("%Y%m%dT%H%M%SZ")));
        fs::create_dir_all(&dir)?;
        if let Some(bytes) = credential {
            write_atomic(&dir.join(".credentials.json"), &bytes)?;
        }
        if let Some(account) = account {
            let bytes = serde_json::to_vec_pretty(&account).map_err(io::Error::other)?;
            write_atomic(&dir.join("oauthAccount.json"), &bytes)?;
        }
        Ok(Some(dir))
    }
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_lowercase()
}
