//! O app do Falcão Token Router para Windows (≙ `Sources/FalcaoTokenRouter`).
//!
//! Apresentação e orquestração: a regra de negócio mora no `router-core`, a
//! marca (o anel) no `gauge-mark`. Aqui ficam a bandeja, as duas janelas (o
//! flyout da bandeja e a janela de Grupos/Ajustes), o laço de rotação, o login
//! por ConPTY e a ponte de comandos com o front em Svelte.

mod locale;

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// O rótulo da janela única (Grupos / Ajustes), num lugar só: é escrito aqui e
/// lido por quem a abre — rótulo repetido em vários arquivos falha em silêncio.
pub const HOME_WINDOW: &str = "home";

/// O que o front precisa saber antes de qualquer outra coisa.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: String,
    locale: locale::Locale,
}

#[tauri::command]
fn app_info(app: AppHandle) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        locale: locale::system(),
    }
}

/// Mostra a janela de Grupos/Ajustes, criando-a se preciso, e a traz para a
/// frente — o app vive na bandeja e não se ativa sozinho.
fn show_home(app: &AppHandle) -> tauri::Result<()> {
    let window = match app.get_webview_window(HOME_WINDOW) {
        Some(window) => window,
        None => WebviewWindowBuilder::new(app, HOME_WINDOW, WebviewUrl::App("index.html".into()))
            .title("Falcão Token Router")
            // Tamanho FIXO (≙ macOS 520×620): as abas têm alturas naturais
            // diferentes e a janela pularia de tamanho a cada troca.
            .inner_size(520.0, 620.0)
            .resizable(false)
            .maximizable(false)
            .build()?,
    };
    window.unminimize()?;
    window.show()?;
    window.set_focus()?;
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        // Primeiro plugin, de propósito: a 2ª execução entrega aqui e sai antes
        // de subir qualquer outra coisa. Abrir a janela é a resposta útil.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = show_home(app);
        }))
        .setup(|app| {
            show_home(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![app_info])
        .run(tauri::generate_context!())
        .expect("o app não conseguiu subir");
}
