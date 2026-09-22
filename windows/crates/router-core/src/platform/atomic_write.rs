//! Escrita atômica: grava num temporário na MESMA pasta e renomeia por cima.
//!
//! Para o `usage/<email>.json` o que importa é não deixar ninguém ler um arquivo
//! meio escrito (o app lê a cada 3 min). O `rename` sobre o destino é atômico no
//! Windows 10+ e, por ser de um arquivo recém-escrito, deixa **mtime novo** — o
//! que a credencial vai exigir na Fase 4 (o Claude Code releva a credencial
//! quando o mtime muda). Nunca um `CopyFile`, que preservaria o mtime da origem.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Grava `bytes` em `path` de forma atômica, criando a pasta se preciso.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dir)?;

    let base = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = dir.join(format!(
        ".{}.tmp-{}-{}",
        base,
        std::process::id(),
        unique_suffix()
    ));

    // Escopo fecha o arquivo (e libera o handle) antes do rename.
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }

    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}
