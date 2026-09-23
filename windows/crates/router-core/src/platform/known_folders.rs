//! A pasta Documentos de verdade (`SHGetKnownFolderPath`), para achar o
//! `$PROFILE` do PowerShell.
//!
//! Não é `%USERPROFILE%\Documents`: com o OneDrive fazendo backup da pasta, ela
//! mora em `%USERPROFILE%\OneDrive\Documentos` (ou o nome localizado) — e é lá que
//! os dois PowerShell procuram o perfil (visto nesta máquina no planejamento).

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use windows_sys::core::PWSTR;
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::UI::Shell::{FOLDERID_Documents, SHGetKnownFolderPath, KF_FLAG_DEFAULT};

/// A pasta Documentos do usuário atual, redirecionada ou não.
pub fn documents_dir() -> Option<PathBuf> {
    let mut raw: PWSTR = std::ptr::null_mut();
    // SAFETY: GUID constante; token nulo = usuário atual; `raw` recebe um buffer
    // do sistema que é liberado com `CoTaskMemFree` logo abaixo.
    let hr = unsafe {
        SHGetKnownFolderPath(
            &FOLDERID_Documents,
            KF_FLAG_DEFAULT as u32,
            std::ptr::null_mut(),
            &mut raw,
        )
    };
    let path = if hr >= 0 && !raw.is_null() {
        // SAFETY: em sucesso, `raw` é uma string UTF-16 terminada em zero.
        let len = (0..).take_while(|&i| unsafe { *raw.add(i) } != 0).count();
        // SAFETY: `len` posições válidas, contadas acima.
        let wide = unsafe { std::slice::from_raw_parts(raw, len) };
        Some(PathBuf::from(OsString::from_wide(wide)))
    } else {
        None
    };
    // SAFETY: liberar nulo é permitido; senão é o buffer que a chamada alocou.
    unsafe { CoTaskMemFree(raw as *const _) };
    path
}
