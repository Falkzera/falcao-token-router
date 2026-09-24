//! A bandeja (≙ `MenuBarLabel` + o `MenuBarExtra` do macOS): o anel desenhado
//! em runtime, o tooltip com janela/origem/idade, o menu do botão direito e o
//! flyout no clique esquerdo.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use chrono::Utc;
use gauge_mark::{render_tray, TaskbarTheme, TrayKey};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::flyout::{self, Area};
use crate::i18n::t;
use crate::state::{AppState, HomeTab};
use crate::system;
use crate::tray_text::{tray_view, GroupStatus, TrayView};

pub const TRAY_ID: &str = "main";

/// O que a bandeja mostra agora, lido do store (sob a trava dele, rápido).
pub fn current_view(state: &AppState) -> TrayView {
    let store = state.store();
    let config = store.config();
    let groups: Vec<GroupStatus> = config
        .groups
        .iter()
        .map(|group| GroupStatus {
            name: &group.name,
            is_default: group.config_dir.is_default,
            active: store
                .active_by_group()
                .get(&group.id)
                .and_then(|id| config.account(*id))
                .map(|account| (account.label(), store.usage_detail().get(&account.id))),
        })
        .collect();
    tray_view(&groups, state.locale, Utc::now())
}

/// Os desenhos já feitos: no máximo 21 passos × 3 cores × 2 temas por tamanho,
/// na vida do processo — sem cache seria um render a cada atualização.
fn icon_for(key: TrayKey) -> Image<'static> {
    static CACHE: OnceLock<Mutex<HashMap<TrayKey, Vec<u8>>>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let rgba = cache
        .entry(key)
        .or_insert_with(|| render_tray(key).rgba)
        .clone();
    Image::new_owned(rgba, key.size, key.size)
}

fn icon(view: &TrayView, theme: TaskbarTheme) -> Image<'static> {
    icon_for(TrayKey::new(view.fraction, theme, system::tray_icon_size()))
}

/// Cria a bandeja. O menu fala no idioma do Windows; as ações reaproveitam as
/// portas do rodapé do painel do macOS (Grupos, Ajustes, Sair).
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let locale = state.locale;
    let groups = MenuItem::with_id(
        app,
        "groups",
        t(locale, "panel.groups", &[]),
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(
        app,
        "settings",
        t(locale, "panel.settings", &[]),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        t(locale, "panel.quit", &[]),
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&groups, &settings, &separator, &quit])?;

    let view = current_view(&state);
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon(&view, system::taskbar_theme()))
        .tooltip(&view.tooltip)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "groups" => crate::show_home(app, HomeTab::Groups),
            "settings" => crate::show_home(app, HomeTab::Settings),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Solta do botão esquerdo, como os flyouts do sistema. O `rect` é
            // o ícone, em pixels físicos (no excedente do Windows 11, o ícone
            // dentro do popup do `^`).
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                let position = rect.position.to_physical::<i32>(1.0);
                let size = rect.size.to_physical::<i32>(1.0);
                flyout::toggle(
                    tray.app_handle(),
                    Area {
                        x: position.x,
                        y: position.y,
                        w: size.width,
                        h: size.height,
                    },
                );
            }
        })
        .build(app)?;
    Ok(())
}

/// Redesenha o anel e reescreve o tooltip com o quadro atual do store (e o
/// tema atual da barra, que o usuário pode ter trocado).
pub fn refresh(app: &AppHandle) {
    let state = app.state::<AppState>();
    let view = current_view(&state);
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(icon(&view, system::taskbar_theme())));
        let _ = tray.set_tooltip(Some(&view.tooltip));
    }
}
