//! O nome 8.3 de um caminho (`C:\PROGRA~1\…`), via `GetShortPathNameW`.
//!
//! Serve à status line: o Claude Code roda o comando pelo Git Bash (ou pelo
//! PowerShell), e o caminho com `/` e SEM aspas é a única forma que funciona nos
//! dois (medido no spike de 22/09/2026). Com espaço no caminho, o nome 8.3 mantém
//! essa forma. O 8.3 vem ligado no volume do sistema, mas pode estar desligado
//! noutros — aí não há nome curto, e quem chama cai nas aspas.

use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;

/// O nome curto de um caminho que EXISTE, ou `None` (inexistente, ou 8.3
/// desligado — nesse caso o Windows devolve o nome longo, e isso conta como
/// "sem nome curto" quando ainda há espaço nele).
pub fn short_path(path: &Path) -> Option<PathBuf> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: `wide` termina em zero; com buffer nulo, a chamada só mede.
    let needed = unsafe { GetShortPathNameW(wide.as_ptr(), std::ptr::null_mut(), 0) };
    if needed == 0 {
        return None;
    }
    let mut buffer = vec![0u16; needed as usize];
    // SAFETY: `buffer` tem `needed` posições, o tamanho que a chamada pediu.
    let written = unsafe { GetShortPathNameW(wide.as_ptr(), buffer.as_mut_ptr(), needed) };
    if written == 0 || written >= needed {
        return None;
    }
    let short = PathBuf::from(OsString::from_wide(&buffer[..written as usize]));
    (!short.to_string_lossy().contains(' ')).then_some(short)
}
