//! O app do Falcão Token Router para Windows (≙ `Sources/FalcaoTokenRouter`).
//!
//! Apresentação e orquestração: a regra de negócio mora no `router-core`, a
//! marca (o anel) no `gauge-mark`. Aqui ficam a bandeja, as duas janelas (o
//! flyout da bandeja e a janela de Grupos/Ajustes), o laço de rotação, o login
//! por ConPTY e a ponte de comandos com o front em Svelte.

mod i18n;
mod locale;
mod rotation_loop;
mod state;
mod system;
mod tray;
mod tray_text;

use std::path::Path;

use router_core::engine::shell_integration::ShellTargets;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder};

use state::{AppState, HomeTab};

/// O rótulo da janela única (Grupos / Ajustes), num lugar só: é escrito aqui e
/// lido por quem a abre — rótulo repetido em vários arquivos falha em silêncio.
pub const HOME_WINDOW: &str = "home";

/// O evento que pede à janela aberta para trocar de aba.
const NAVIGATE: &str = "navigate";

/// O que o front precisa saber antes de qualquer outra coisa.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: String,
    locale: locale::Locale,
    /// A aba com que a janela abre (a bandeja pode ter pedido Ajustes).
    initial_tab: HomeTab,
}

#[tauri::command]
fn app_info(app: AppHandle, state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        locale: state.locale,
        initial_tab: state.take_tab(),
    }
}

/// Mostra a janela de Grupos/Ajustes na aba pedida, criando-a se preciso, e a
/// traz para a frente — o app vive na bandeja e não se ativa sozinho.
pub fn show_home(app: &AppHandle, tab: HomeTab) {
    if let Some(window) = app.get_webview_window(HOME_WINDOW) {
        let _ = window.emit(NAVIGATE, tab);
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let state = app.state::<AppState>();
    state.request_tab(tab);
    let built = WebviewWindowBuilder::new(app, HOME_WINDOW, WebviewUrl::App("index.html".into()))
        .title(i18n::t(state.locale, "home.title", &[]))
        // Tamanho FIXO (≙ macOS 520×620): as abas têm alturas naturais
        // diferentes e a janela pularia de tamanho a cada troca.
        .inner_size(520.0, 620.0)
        .resizable(false)
        .maximizable(false)
        .build();
    if let Ok(window) = built {
        let _ = window.set_focus();
    }
}

/// Liga o app ao `router.exe` ao lado dele e cura a integração de terminal se
/// ela aponta para outro lugar (app movido ou reinstalado). O modo de falha é
/// silencioso — `claude trabalho` cairia no `claude` puro, na conta errada —, e
/// a subida é o único momento em que se sabe onde o binário está AGORA.
fn attach_router(state: &AppState) {
    let mut store = state.store();
    store.set_router_path(system::router_path());
    let targets = ShellTargets::for_user(Path::new(&state.home));
    store.heal_shell_integration(&targets, system::status_shell());
}

pub fn run() {
    let app = tauri::Builder::default()
        // Primeiro plugin, de propósito: a 2ª execução entrega aqui e sai antes
        // de subir qualquer outra coisa. Abrir a janela é a resposta útil.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_home(app, HomeTab::Groups);
        }))
        .setup(|app| {
            let state = AppState::open();
            attach_router(&state);
            // A janela abre sozinha só quando não há o que mostrar na bandeja
            // ainda (1ª execução, nenhum grupo) — decidido uma vez, na subida.
            let first_run = state.store().config().groups.is_empty();
            app.manage(state);
            tray::create(app.handle())?;
            rotation_loop::start(app.handle().clone());
            if first_run {
                show_home(app.handle(), HomeTab::Groups);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![app_info])
        .build(tauri::generate_context!())
        .expect("o app não conseguiu subir");

    app.run(|_app, event| {
        // Fechar a janela não encerra o app: ele vive na bandeja. Só o "Sair"
        // (que pede a saída com código) encerra.
        if let RunEvent::ExitRequested {
            code: None, api, ..
        } = event
        {
            api.prevent_exit();
        }
    });
}
