//! Acrescentar um bloco ao perfil de um shell **em bytes**, sem nunca reescrever
//! o arquivo.
//!
//! No macOS um `~/.zshrc` que não decodificava em UTF-8 virava "vazio" e era
//! sobrescrito só com a linha nova — o arquivo inteiro do usuário, perdido. O
//! Windows tem mais jeitos de um perfil não ser UTF-8: o Windows PowerShell 5.1
//! grava "Unicode" (UTF-16LE com BOM), editores antigos gravam ANSI. Então:
//!
//! - a codificação é detectada pelo BOM e o bloco é codificado nela (UTF-16LE,
//!   UTF-16BE, UTF-8); sem BOM, o bloco é ASCII — igual em UTF-8 e em ANSI;
//! - o fim de linha é o que o arquivo já usa (CRLF ou LF);
//! - o arquivo é aberto em modo append: nada do que existia é tocado, e um link
//!   para o perfil real é seguido, não substituído;
//! - arquivo que existe e não pode ser lido faz a operação falhar — nunca vira
//!   "vazio".

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use super::atomic_write::{read_retrying, retrying};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppendOutcome {
    Added,
    AlreadyPresent,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Encoding {
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    /// Sem BOM: UTF-8 ou ANSI. O bloco é ASCII, igual nos dois.
    Plain,
}

fn detect(bytes: &[u8]) -> (Encoding, &[u8]) {
    match bytes {
        [0xEF, 0xBB, 0xBF, rest @ ..] => (Encoding::Utf8Bom, rest),
        [0xFF, 0xFE, rest @ ..] => (Encoding::Utf16Le, rest),
        [0xFE, 0xFF, rest @ ..] => (Encoding::Utf16Be, rest),
        _ => (Encoding::Plain, bytes),
    }
}

fn decode(encoding: Encoding, body: &[u8]) -> String {
    let units = |be: bool| -> Vec<u16> {
        body.as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                if be {
                    u16::from_be_bytes(*pair)
                } else {
                    u16::from_le_bytes(*pair)
                }
            })
            .collect()
    };
    match encoding {
        Encoding::Utf16Le => String::from_utf16_lossy(&units(false)),
        Encoding::Utf16Be => String::from_utf16_lossy(&units(true)),
        // ANSI decodificado como UTF-8 com perda: os bytes não-ASCII viram �,
        // mas o que se procura (o marcador) é ASCII e sobrevive.
        Encoding::Utf8Bom | Encoding::Plain => String::from_utf8_lossy(body).into_owned(),
    }
}

fn encode(encoding: Encoding, text: &str) -> Vec<u8> {
    match encoding {
        Encoding::Utf16Le => text.encode_utf16().flat_map(u16::to_le_bytes).collect(),
        Encoding::Utf16Be => text.encode_utf16().flat_map(u16::to_be_bytes).collect(),
        Encoding::Utf8Bom | Encoding::Plain => text.as_bytes().to_vec(),
    }
}

/// Acrescenta `lines` ao arquivo se `marker` ainda não está nele (sem caixa).
/// Arquivo ausente é criado (com a pasta), em ASCII, com o fim de linha
/// `default_eol`.
pub fn append_block(
    path: &Path,
    marker: &str,
    lines: &[&str],
    default_eol: &str,
) -> io::Result<AppendOutcome> {
    let existing = match read_retrying(path) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };

    let Some(bytes) = existing else {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        let mut block = lines.join(default_eol);
        block.push_str(default_eol);
        fs::write(path, block.as_bytes())?;
        return Ok(AppendOutcome::Added);
    };

    let (encoding, body) = detect(&bytes);
    let text = decode(encoding, body);
    if text.to_lowercase().contains(&marker.to_lowercase()) {
        return Ok(AppendOutcome::AlreadyPresent);
    }
    let eol = if text.contains("\r\n") {
        "\r\n"
    } else if text.contains('\n') {
        "\n"
    } else {
        default_eol
    };

    let mut block = String::new();
    if !text.is_empty() && !text.ends_with('\n') {
        block.push_str(eol); // a última linha do usuário não gruda no bloco
    }
    block.push_str(eol);
    block.push_str(&lines.join(eol));
    block.push_str(eol);

    let payload = encode(encoding, &block);
    let mut file = retrying(|| OpenOptions::new().append(true).open(path))?;
    file.write_all(&payload)?;
    file.sync_all()?;
    Ok(AppendOutcome::Added)
}
