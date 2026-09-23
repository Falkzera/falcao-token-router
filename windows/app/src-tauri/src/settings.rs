//! Os ajustes do app (≙ a parte "Sistema" do SettingsView do macOS) e as
//! portas para fora dele.
//!
//! - "Abrir no login": pelo plugin de autostart (HKCU\…\Run). O estado exibido é
//!   SEMPRE relido do sistema, nunca de uma preferência nossa (≙ LoginItem):
//!   um checkbox marcado sobre um registro que falhou é pior que nenhum.
//! - "Mostrar na barra de tarefas" (≙ "Mostrar no Dock", pela mesma razão: o
//!   Windows 11 esconde ícone novo da bandeja no excedente `^`, e aí não há por
//!   onde abrir o app): com ele, a janela abre na subida, e dá para fixar o
//!   botão dela na barra de tarefas. O X não depende dele: fecha a janela e o
//!   app fica na bandeja (até 23/09/2026, com ele ligado, o X só minimizava).
//! - O último tamanho da janela Grupos/Ajustes (`home_window`): acompanhado na
//!   memória a cada `Resized` (`remember`) e gravado ao fechar a janela e na
//!   saída do app (`flush`).
//! - `open_url` só abre o que está na lista: as páginas de Configurações que a
//!   tela sugere, o logout do claude.ai e o login oficial.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use router_core::platform::atomic_write::{read_retrying, write_atomic};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

use crate::home_window::WindowSize;

#[derive(Clone, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub show_in_taskbar: bool,
    /// O último tamanho da janela Grupos/Ajustes (ver `home_window`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home_window: Option<WindowSize>,
}

impl AppSettings {
    /// A janela abre sozinha na subida (≙ `presentAtLaunch` do macOS)? Quando
    /// não há o que mostrar na bandeja ainda (nenhum grupo) ou quando o usuário
    /// pediu a barra de tarefas — aí ele espera um app comum, e app comum abre
    /// janela (e o botão da barra só existe com ela). Decidido UMA vez, na
    /// subida: reavaliar depois faria a janela reaparecer no meio do uso.
    pub fn present_at_launch(&self, no_groups: bool) -> bool {
        no_groups || self.show_in_taskbar
    }
}

pub struct SettingsStore {
    path: Option<PathBuf>,
    value: Mutex<AppSettings>,
    /// A memória tem o que o disco ainda não tem (`remember` sem `flush`).
    pending: AtomicBool,
    /// Por que o sistema recusou o registro de "abrir no login" (a tela mostra).
    autostart_failure: Mutex<Option<String>>,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|p| p.into_inner())
}

impl SettingsStore {
    /// `<%APPDATA%>\com.synqo.falcao-token-router\settings.json` — preferência
    /// de interface (pode viajar com o perfil; credencial nenhuma mora aqui).
    pub fn open(app: &AppHandle) -> Self {
        Self::at(
            app.path()
                .app_config_dir()
                .ok()
                .map(|d| d.join("settings.json")),
        )
    }

    /// Num arquivo dado (os testes usam um temporário). Sem caminho, vive só
    /// em memória. Ilegível = o padrão: é preferência de interface, e o
    /// arquivo é só nosso.
    pub fn at(path: Option<PathBuf>) -> Self {
        let value = path
            .as_ref()
            .and_then(|p| read_retrying(p).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        SettingsStore {
            path,
            value: Mutex::new(value),
            pending: AtomicBool::new(false),
            autostart_failure: Mutex::new(None),
        }
    }

    pub fn get(&self) -> AppSettings {
        lock(&self.value).clone()
    }

    /// Muda só a memória; `flush` grava. Para o que muda a cada passo — o
    /// tamanho da janela durante o arrasto: gravar a cada `Resized` seria um
    /// arquivo por quadro.
    pub fn remember(&self, change: impl FnOnce(&mut AppSettings)) {
        let mut value = lock(&self.value);
        let before = value.clone();
        change(&mut value);
        if *value != before {
            self.pending.store(true, Ordering::Relaxed);
        }
    }

    /// Grava o que `remember` mudou (nada, se nada mudou).
    pub fn flush(&self) -> std::io::Result<()> {
        let value = lock(&self.value);
        if !self.pending.load(Ordering::Relaxed) {
            return Ok(());
        }
        self.write(&value)
    }

    fn update(&self, change: impl FnOnce(&mut AppSettings)) -> std::io::Result<()> {
        let mut value = lock(&self.value);
        change(&mut value);
        self.write(&value)
    }

    /// Grava o valor inteiro — com ele vai também o que só estava na memória.
    /// Sempre com a trava do valor na mão: ninguém o muda no meio.
    fn write(&self, value: &AppSettings) -> std::io::Result<()> {
        if let Some(path) = &self.path {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let bytes = serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?;
            write_atomic(path, &bytes)?;
        }
        self.pending.store(false, Ordering::Relaxed);
        Ok(())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    /// Lido do sistema agora.
    pub autostart: bool,
    pub autostart_failure: Option<String>,
    pub show_in_taskbar: bool,
    pub version: String,
}

fn view(app: &AppHandle, store: &SettingsStore) -> SettingsView {
    SettingsView {
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        autostart_failure: lock(&store.autostart_failure).clone(),
        show_in_taskbar: store.get().show_in_taskbar,
        version: app.package_info().version.to_string(),
    }
}

#[tauri::command]
pub fn get_settings(app: AppHandle, store: State<'_, SettingsStore>) -> SettingsView {
    view(&app, &store)
}

/// Liga/desliga "abrir no login" e confere contra o sistema em vez de assumir
/// que deu certo.
#[tauri::command]
pub fn set_autostart(app: AppHandle, store: State<'_, SettingsStore>, on: bool) -> SettingsView {
    let manager = app.autolaunch();
    let result = if on {
        manager.enable()
    } else {
        manager.disable()
    };
    *lock(&store.autostart_failure) = result.err().map(|e| e.to_string());
    view(&app, &store)
}

#[tauri::command]
pub fn set_show_in_taskbar(
    app: AppHandle,
    store: State<'_, SettingsStore>,
    on: bool,
) -> Result<SettingsView, String> {
    store
        .update(|s| s.show_in_taskbar = on)
        .map_err(|e| e.to_string())?;
    Ok(view(&app, &store))
}

/// As únicas portas para fora: as páginas de Configurações que a tela sugere, o
/// logout do claude.ai (login que voltou na conta errada) e o login oficial
/// (o link que o próprio `claude auth login` imprime).
pub fn allowed_url(url: &str) -> bool {
    const EXACT: &[&str] = &[
        "ms-settings:developers",
        "ms-settings:taskbar",
        "https://claude.ai/logout",
    ];
    const PREFIXES: &[&str] = &["https://claude.com/", "https://platform.claude.com/"];
    EXACT.contains(&url) || PREFIXES.iter().any(|p| url.starts_with(p))
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    if !allowed_url(&url) {
        return Err(format!("endereço fora da lista: {url}"));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_survive_a_restart() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp
            .path()
            .join("com.synqo.falcao-token-router")
            .join("settings.json");
        let store = SettingsStore::at(Some(path.clone()));
        assert!(!store.get().show_in_taskbar, "desligado de fábrica");

        store.update(|s| s.show_in_taskbar = true).unwrap();
        assert!(SettingsStore::at(Some(path)).get().show_in_taskbar);
    }

    /// O arquivo é só nosso e guarda preferência de interface: ilegível (meio
    /// escrito, editado à mão) volta ao padrão em vez de derrubar o app.
    #[test]
    fn an_unreadable_file_falls_back_to_the_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, b"{ nao e json").unwrap();
        assert_eq!(SettingsStore::at(Some(path)).get(), AppSettings::default());
    }

    /// ≙ `presentAtLaunch` do macOS: a janela abre sozinha quando não há o que
    /// mostrar na bandeja (nenhum grupo) ou quando o app deve estar na barra de
    /// tarefas — e a decisão é da subida.
    /// O tamanho da janela muda a cada passo do arrasto: fica na memória e só
    /// vai para o disco no `flush` (ao fechar a janela e na saída do app).
    #[test]
    fn the_window_size_reaches_the_disk_only_on_flush() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        let store = SettingsStore::at(Some(path.clone()));
        store.flush().unwrap();
        assert!(!path.exists(), "nada mudou: nada a gravar");

        let size = WindowSize {
            width: 900.0,
            height: 700.0,
            maximized: true,
        };
        store.remember(|s| s.home_window = Some(size));
        assert_eq!(store.get().home_window, Some(size), "a memória já sabe");
        assert_eq!(
            SettingsStore::at(Some(path.clone())).get().home_window,
            None
        );

        store.flush().unwrap();
        assert_eq!(
            SettingsStore::at(Some(path.clone())).get().home_window,
            Some(size)
        );

        // Uma preferência gravada depois leva junto o que estava só na memória.
        let resized = WindowSize {
            width: 640.0,
            ..size
        };
        store.remember(|s| s.home_window = Some(resized));
        store.update(|s| s.show_in_taskbar = true).unwrap();
        let reread = SettingsStore::at(Some(path)).get();
        assert!(reread.show_in_taskbar);
        assert_eq!(reread.home_window, Some(resized));
    }

    /// O `settings.json` de antes do tamanho guardado continua valendo.
    #[test]
    fn a_settings_file_from_before_the_window_size_still_reads() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, b"{\n  \"showInTaskbar\": true\n}").unwrap();
        let settings = SettingsStore::at(Some(path)).get();
        assert!(settings.show_in_taskbar);
        assert_eq!(settings.home_window, None);
    }

    #[test]
    fn the_window_opens_at_launch_without_groups_or_with_the_taskbar_option() {
        let plain = AppSettings::default();
        let taskbar = AppSettings {
            show_in_taskbar: true,
            ..AppSettings::default()
        };
        assert!(plain.present_at_launch(true));
        assert!(!plain.present_at_launch(false));
        assert!(taskbar.present_at_launch(false));
        assert!(taskbar.present_at_launch(true));
    }

    #[test]
    fn only_the_listed_urls_open() {
        assert!(allowed_url("ms-settings:developers"));
        assert!(allowed_url("ms-settings:taskbar"));
        assert!(allowed_url("https://claude.ai/logout"));
        assert!(allowed_url(
            "https://claude.com/cai/oauth/authorize?code=true&client_id=x"
        ));
        assert!(allowed_url(
            "https://platform.claude.com/oauth/authorize?x=1"
        ));

        assert!(!allowed_url("https://claude.com.exemplo.com/"));
        assert!(!allowed_url("http://claude.com/"));
        assert!(!allowed_url("file:///C:/Windows/System32/calc.exe"));
        assert!(!allowed_url("ms-settings:privacy"));
        assert!(!allowed_url("https://claude.ai/logout/../outra"));
    }
}
