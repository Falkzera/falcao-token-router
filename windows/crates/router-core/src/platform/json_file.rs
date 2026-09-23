//! Edição cirúrgica de um arquivo JSON cujo topo é um objeto (`.claude.json`,
//! `settings.json`).
//!
//! Lê o que houver, deixa o chamador mexer só nas chaves dele e regrava
//! preservando todas as outras, na ordem em que estavam (o `serde_json` com
//! `preserve_order`), com recuo de 2 espaços como o Claude Code grava. Um
//! arquivo existente e ilegível é **recusado** — nunca substituído: no macOS ele
//! virava `{}` e o conteúdo do usuário se perdia. Arquivo ausente ou vazio vira
//! um objeto novo (não há o que perder).

use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::atomic_write::{read_retrying, write_atomic};

#[derive(Debug, thiserror::Error)]
pub enum JsonFileError {
    /// O arquivo existe e não é um objeto JSON legível. Nada foi escrito.
    #[error("{} ilegível ({reason}) — recusado, nada foi escrito", path.display())]
    Unreadable { path: PathBuf, reason: String },
    #[error("falha ao gravar {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// O objeto raiz de um JSON existente, ou o motivo de ele não servir.
fn parse_root(bytes: &[u8]) -> Result<Map<String, Value>, String> {
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return Ok(Map::new());
    }
    match serde_json::from_slice::<Value>(bytes) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err("a raiz não é um objeto JSON".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// Lê o objeto (ausente = vazio), aplica `edit` e regrava de forma atômica.
pub fn edit_object(
    path: &Path,
    edit: impl FnOnce(&mut Map<String, Value>),
) -> Result<(), JsonFileError> {
    let (mut root, trailing_newline) = match read_retrying(path) {
        Ok(bytes) => {
            let root = parse_root(&bytes).map_err(|reason| JsonFileError::Unreadable {
                path: path.to_path_buf(),
                reason,
            })?;
            (root, bytes.ends_with(b"\n"))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => (Map::new(), false),
        // Existe e não deu para ler: recusa, pelo mesmo motivo do ilegível.
        Err(e) => {
            return Err(JsonFileError::Unreadable {
                path: path.to_path_buf(),
                reason: e.to_string(),
            })
        }
    };

    edit(&mut root);

    let mut bytes = serde_json::to_vec_pretty(&Value::Object(root))
        .expect("um serde_json::Value sempre serializa");
    if trailing_newline {
        bytes.push(b'\n');
    }
    write_atomic(path, &bytes).map_err(|source| JsonFileError::Io {
        path: path.to_path_buf(),
        source,
    })
}
