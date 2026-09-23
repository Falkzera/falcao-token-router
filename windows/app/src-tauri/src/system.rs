//! O que o app pergunta ao Windows: o tema da barra de tarefas, o tamanho do
//! ícone da bandeja, e onde mora o `router.exe`.

use std::path::{Path, PathBuf};

use gauge_mark::TaskbarTheme;
use router_core::engine::shell_integration::StatusShell;
use router_core::platform::git_bash::{find_git_bash, GitBashEnv};
use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows_sys::Win32::UI::HiDpi::{GetDpiForSystem, GetSystemMetricsForDpi};
use windows_sys::Win32::UI::WindowsAndMessaging::SM_CXSMICON;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// O tema da BARRA DE TAREFAS — que no Windows 11 é escolhido à parte do tema
/// dos apps ("modo do Windows" × "modo dos apps"). Sem o valor, escura (o
/// padrão histórico da barra).
pub fn taskbar_theme() -> TaskbarTheme {
    let key = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let value = wide("SystemUsesLightTheme");
    let mut data: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: chave e valor terminam em zero; `data`/`size` são nossos e do
    // tamanho de um DWORD, o único tipo aceito pela flag.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut data as *mut u32).cast(),
            &mut size,
        )
    };
    if status == 0 && data == 1 {
        TaskbarTheme::Light
    } else {
        TaskbarTheme::Dark
    }
}

/// O tamanho em que o Windows desenha ícone pequeno na escala do sistema (16 px
/// a 100%, 20 a 125%, 24 a 150%, 32 a 200%). Desenhar no tamanho exato evita o
/// anel borrado de uma imagem redimensionada.
pub fn tray_icon_size() -> u32 {
    // SAFETY: funções sem ponteiro; só leem a configuração de exibição.
    let size = unsafe { GetSystemMetricsForDpi(SM_CXSMICON, GetDpiForSystem()) };
    u32::try_from(size).ok().filter(|s| *s >= 16).unwrap_or(16)
}

/// O `router.exe` irmão de um executável (≙ `RouterBinary.swift`).
pub fn router_beside(exe: &Path) -> Option<PathBuf> {
    let sibling = exe.parent()?.join("router.exe");
    sibling.is_file().then_some(sibling)
}

/// O `router.exe` que a integração e a status line citam: ao lado do app. No
/// instalado, o sidecar do NSIS (onde ele cai é conferido na fase 6); em
/// desenvolvimento, `cargo build --workspace` põe os dois em `target\debug\`.
pub fn router_path() -> Option<PathBuf> {
    router_beside(&std::env::current_exe().ok()?)
}

/// O shell que o Claude Code vai usar para a status line (a mesma busca dele).
pub fn status_shell() -> StatusShell {
    if find_git_bash(&GitBashEnv::from_process()).is_some() {
        StatusShell::Bash
    } else {
        StatusShell::PowerShell
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_router_is_found_only_beside_the_app() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join("FalcaoTokenRouter.exe");
        assert_eq!(router_beside(&exe), None);

        std::fs::write(tmp.path().join("router.exe"), b"").unwrap();
        assert_eq!(router_beside(&exe), Some(tmp.path().join("router.exe")));
    }
}
