//! Escrita atômica e E/S de arquivo com nova tentativa.
//!
//! **Escrita atômica:** grava num temporário na MESMA pasta e renomeia por cima.
//! Ninguém lê um arquivo meio escrito, e — o que decide a troca a quente — o
//! destino fica com **mtime novo**: o Claude Code só relê o `.credentials.json`
//! quando o mtime muda (visto no JS do binário 2.1.280 e confirmado no spike de
//! 22/09/2026). Por isso nunca um `CopyFile`, que preservaria o mtime da origem.
//!
//! **Nova tentativa:** no Windows um arquivo aberto por outro processo sem
//! compartilhamento (antivírus, indexador, o próprio Claude Code no meio de uma
//! escrita) faz `rename`/`read`/`remove` falharem na hora. É passageiro: espera
//! um pouco e tenta de novo, com prazo curto — persistindo, o erro sobe.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Esperas entre as tentativas, em ms: ~0,6 s no total. Longo para um antivírus
/// soltar o arquivo, curto para ninguém achar que travou.
const BACKOFF_MS: [u64; 7] = [5, 10, 20, 40, 80, 160, 320];

/// Códigos Win32 de "outro processo está com o arquivo agora":
/// `ERROR_ACCESS_DENIED` (5 — é o que o `rename` devolve sobre um destino aberto
/// sem `FILE_SHARE_DELETE`, e também o de um arquivo com exclusão pendente),
/// `ERROR_SHARING_VIOLATION` (32), `ERROR_LOCK_VIOLATION` (33) e
/// `ERROR_USER_MAPPED_FILE` (1224).
const TRANSIENT_CODES: [i32; 4] = [5, 32, 33, 1224];

/// O erro é do tipo "espere e tente de novo"?
pub fn is_transient(e: &io::Error) -> bool {
    e.raw_os_error()
        .is_some_and(|code| TRANSIENT_CODES.contains(&code))
}

/// Roda `op`, repetindo enquanto o erro for passageiro (ver [`is_transient`]).
pub fn retrying<T>(mut op: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let mut result = op();
    for wait in BACKOFF_MS {
        match &result {
            Err(e) if is_transient(e) => {
                thread::sleep(Duration::from_millis(wait));
                result = op();
            }
            _ => break,
        }
    }
    result
}

/// Lê o arquivo inteiro, esperando quem o segura sem compartilhar.
pub fn read_retrying(path: &Path) -> io::Result<Vec<u8>> {
    retrying(|| fs::read(path))
}

/// Apaga o arquivo, esperando quem o segura. Ausente já é o resultado pedido.
pub fn remove_retrying(path: &Path) -> io::Result<()> {
    match retrying(|| fs::remove_file(path)) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Distingue dois temporários criados no mesmo instante pelo mesmo processo.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_path(dir: &Path, target: &Path) -> PathBuf {
    let base = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    dir.join(format!(".{base}.tmp-{}-{nanos}-{n}", std::process::id()))
}

/// Grava `bytes` em `path` de forma atômica, criando a pasta se preciso. O
/// destino sai com mtime novo. Em erro, o original fica intacto e nenhum
/// temporário sobra.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    fs::create_dir_all(dir)?;

    let tmp = temp_path(dir, path);
    // Escopo fecha o arquivo (e libera o handle) antes do rename.
    let written = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()
    })();
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }

    match retrying(|| fs::rename(&tmp, path)) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}
