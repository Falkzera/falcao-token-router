//! O app do Falcão Token Router para Windows (≙ `Sources/FalcaoTokenRouter`).
//!
//! Apresentação e orquestração: a regra de negócio mora no `router-core`, a
//! marca (o anel) no `gauge-mark`. Aqui ficam a bandeja, as duas janelas (o
//! flyout da bandeja e a janela de Grupos/Ajustes), o laço de rotação, o login
//! por ConPTY e a ponte de comandos com o front em Svelte.

mod commands;
mod flyout;
mod home_window;
mod i18n;
mod locale;
mod login;
mod login_output;
mod login_session;
mod rotation_loop;
mod settings;
mod snapshot;
mod state;
mod status_line;
mod system;
mod terminal;
mod tray;
mod tray_text;

use std::path::Path;

use router_core::engine::shell_integration::ShellTargets;
use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};

use settings::SettingsStore;
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

/// O quadro atual (grupos, contas, uso com procedência, sessões, erro).
#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> snapshot::Snapshot {
    snapshot::build(&state.store(), state.measuring(), state.locale)
}

/// O front mediu o conteúdo do flyout: a janela acompanha a altura.
#[tauri::command]
fn fit_flyout(app: AppHandle, height: f64) {
    flyout::fit_height(&app, height);
}

/// Uma porta do rodapé do flyout: fecha o flyout e abre a janela na aba.
#[tauri::command]
fn open_home(app: AppHandle, tab: HomeTab) {
    flyout::hide(&app);
    show_home(&app, tab);
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// Copia um texto (o comando do grupo, o link do login) para a área de
/// transferência do Windows.
#[tauri::command]
fn copy_text(app: AppHandle, text: String) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().write_text(text).map_err(|e| e.to_string())
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
    let settings = app.state::<SettingsStore>();
    let size = home_window::WindowSize::opening(settings.get().home_window);
    // O acompanhamento parte do tamanho de abertura: aberta maximizada, é para
    // ele que ela volta.
    settings.remember(|s| s.home_window = Some(size));
    let built = WebviewWindowBuilder::new(app, HOME_WINDOW, WebviewUrl::App("index.html".into()))
        .title(i18n::t(state.locale, "home.title", &[]))
        // O tamanho é do usuário, nunca do conteúdo: as abas têm alturas
        // naturais diferentes e a janela pularia a cada troca. Abre no último
        // tamanho (na 1ª vez, o 520×620 de sempre), centralizada e encolhida
        // para caber na área útil — ver `home_window`.
        .inner_size(size.width, size.height)
        .min_inner_size(home_window::MIN_WIDTH, home_window::MIN_HEIGHT)
        .maximized(size.maximized)
        .center()
        .prevent_overflow_with_margin(tauri::LogicalSize::new(
            home_window::SCREEN_MARGIN,
            home_window::SCREEN_MARGIN,
        ))
        .build();
    if let Ok(window) = built {
        let handle = window.clone();
        // O X fecha a janela de verdade, com ou sem "Mostrar na barra de
        // tarefas": o botão sai da barra e o app segue na bandeja, como os
        // apps do excedente (`^`); minimizar fica o do Windows. Até
        // 23/09/2026, com a opção, o X só minimizava (≙ o ícone do Dock, que
        // sobrevive à janela) — mas no Windows o botão da barra é da janela,
        // e quem fecha um app de bandeja espera vê-lo sumir de lá.
        window.on_window_event(move |event| match event {
            WindowEvent::Resized(size) => {
                home_window::track(&handle, &handle.state::<SettingsStore>(), *size);
            }
            // O tamanho vai para o disco ao fechar; a saída do app ("Sair"),
            // que não passa por aqui, grava no `run`.
            WindowEvent::CloseRequested { .. } => {
                let _ = handle.state::<SettingsStore>().flush();
            }
            // A tela do login mora na janela: sem ela, ninguém veria o
            // desfecho — o `claude` é encerrado e a casa reservada limpa
            // (espera o processo sair: fora da thread da interface).
            WindowEvent::Destroyed => {
                let app = handle.app_handle().clone();
                std::thread::spawn(move || login::close_quietly(&app));
            }
            _ => {}
        });
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
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let state = AppState::open();
            attach_router(&state);
            let settings = SettingsStore::open(app.handle());
            // Decidido uma vez, na subida: sem grupo nenhum (a bandeja não tem
            // o que mostrar) ou com o app na barra de tarefas.
            let present = settings
                .get()
                .present_at_launch(state.store().config().groups.is_empty());
            app.manage(state);
            app.manage(settings);
            app.manage(login::LoginState::default());
            app.manage(flyout::FlyoutState::default());
            flyout::create(app.handle())?;
            tray::create(app.handle())?;
            rotation_loop::start(app.handle().clone());
            if present {
                show_home(app.handle(), HomeTab::Groups);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            get_snapshot,
            fit_flyout,
            open_home,
            quit_app,
            copy_text,
            commands::add_group,
            commands::rename_group,
            commands::set_auto_rotate,
            commands::set_threshold,
            commands::reorder_accounts,
            commands::remove_group,
            commands::make_default,
            commands::clear_default,
            commands::activate_account,
            commands::remove_account,
            commands::dismiss_error,
            commands::foreign_default_login,
            commands::measure_group,
            terminal::terminal_report,
            terminal::install_integration,
            terminal::allow_profiles_for,
            settings::get_settings,
            settings::set_autostart,
            settings::set_show_in_taskbar,
            settings::open_url,
            status_line::get_status_line,
            status_line::set_status_line,
            status_line::test_status_line,
            login::start_login,
            login::start_relogin,
            login::current_login,
            login::login_submit_code,
            login::login_retry,
            login::login_recheck,
            login::login_close
        ])
        .build(tauri::generate_context!())
        .expect("o app não conseguiu subir");

    app.run(|app, event| match event {
        // Fechar a janela não encerra o app: ele vive na bandeja. Só o "Sair"
        // (que pede a saída com código) encerra.
        RunEvent::ExitRequested {
            code: None, api, ..
        } => api.prevent_exit(),
        // O "Sair" com a janela aberta não passa pelo fechar dela: o último
        // tamanho vai para o disco aqui.
        RunEvent::Exit => {
            if let Some(settings) = app.try_state::<SettingsStore>() {
                let _ = settings.flush();
            }
        }
        _ => {}
    });
}
