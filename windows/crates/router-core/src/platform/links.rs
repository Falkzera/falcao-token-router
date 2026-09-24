//! Links do Windows para o compartilhamento de perfil: junction e symlink.
//!
//! - **Junction** (pastas): não pede privilégio nem Developer Mode, e o Claude
//!   Code lê e grava através dela como numa pasta comum (o `projects` do grupo
//!   vira o `~\.claude\projects`, e o `--resume` enxerga tudo).
//! - **Symlink** de arquivo: sem privilégio de admin só com o Developer Mode
//!   ligado (a flag `ALLOW_UNPRIVILEGED_CREATE`); sem ele a criação falha com
//!   `ERROR_PRIVILEGE_NOT_HELD`, e quem chama cai no plano B.
//! - **Hardlink: não.** O binário 2.1.280 poda o `history.jsonl` reescrevendo o
//!   arquivo quando ele é comum (e pula quando é link) — um hardlink viraria
//!   dois arquivos diferentes em silêncio.

use std::fs;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{
    CreateSymbolicLinkW, SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE,
};
use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD};

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

fn wide_str(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

/// Cria a junction `link` → `target` (uma pasta que existe).
pub fn junction(target: &Path, link: &Path) -> io::Result<()> {
    ::junction::create(target, link)
}

/// `path` é uma junction?
pub fn is_junction(path: &Path) -> bool {
    ::junction::exists(path).unwrap_or(false)
}

/// `path` é um symlink (e não uma junction)?
pub fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) && !is_junction(path)
}

/// Cria o symlink de ARQUIVO `link` → `target`, sem pedir privilégio. Falha com
/// `ERROR_PRIVILEGE_NOT_HELD` (1314) quando o Developer Mode está desligado e o
/// processo não é admin.
pub fn symlink_file(target: &Path, link: &Path) -> io::Result<()> {
    let (link_w, target_w) = (wide(link), wide(target));
    // SAFETY: as duas strings UTF-16 terminam em zero e vivem até o fim da chamada.
    let ok = unsafe {
        CreateSymbolicLinkW(
            link_w.as_ptr(),
            target_w.as_ptr(),
            SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE,
        )
    };
    if ok {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// O Developer Mode está ligado? (Para a UI e o `doctor` sugerirem ligá-lo;
/// quem decide de verdade é a tentativa de criar o symlink.)
pub fn developer_mode_enabled() -> bool {
    let key = wide_str(r"SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock");
    let value = wide_str("AllowDevelopmentWithoutDevLicense");
    let mut data: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: chave e valor terminam em zero; `data`/`size` são nossos e do
    // tamanho de um DWORD, o único tipo aceito pela flag.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut data as *mut u32).cast(),
            &mut size,
        )
    };
    status == 0 && data == 1
}
