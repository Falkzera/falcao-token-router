//! O flyout da bandeja (≙ o painel do `MenuBarExtra`): 330 px de largura, sem
//! borda, some ao perder o foco. Aparece junto do ícone — acima da barra de
//! tarefas embaixo, abaixo dela em cima, ao lado nas laterais, e perto do
//! clique quando o ícone mora no excedente (`^`) do Windows 11.
//!
//! A janela é criada escondida na subida, para o primeiro clique ser imediato;
//! a altura segue o conteúdo (o front mede e pede).

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};

pub const FLYOUT_WINDOW: &str = "flyout";
/// A largura do painel do macOS.
pub const WIDTH: f64 = 330.0;
/// Folga entre o flyout e a barra/borda da tela (a dos flyouts do sistema).
const GAP: i32 = 12;
/// Um clique no ícone logo depois de o flyout sumir por perda de foco é o
/// MESMO gesto de fechar: o clique tirou o foco antes de chegar aqui.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

/// Um retângulo em pixels físicos.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Area {
    fn right(&self) -> i32 {
        self.x + self.w
    }
    fn bottom(&self) -> i32 {
        self.y + self.h
    }
    fn center_x(&self) -> i32 {
        self.x + self.w / 2
    }
    fn center_y(&self) -> i32 {
        self.y + self.h / 2
    }
}

/// Onde o flyout de tamanho (`w`, `h`) fica, dado o ícone e a área útil do
/// monitor (a tela sem a barra de tarefas).
pub fn position(icon: Area, work: Area, w: i32, h: i32) -> (i32, i32) {
    let (x, y) = if icon.y >= work.bottom() {
        // Barra embaixo (o normal): acima dela, centrado no ícone.
        (icon.center_x() - w / 2, work.bottom() - h - GAP)
    } else if icon.bottom() <= work.y {
        (icon.center_x() - w / 2, work.y + GAP)
    } else if icon.x >= work.right() {
        (work.right() - w - GAP, icon.center_y() - h / 2)
    } else if icon.right() <= work.x {
        (work.x + GAP, icon.center_y() - h / 2)
    } else {
        // O ícone está DENTRO da área útil: é o excedente do Windows 11, que
        // abre acima da barra. Acima do clique se couber; senão, abaixo.
        let above = icon.y - h - GAP;
        (
            icon.center_x() - w / 2,
            if above >= work.y {
                above
            } else {
                icon.bottom() + GAP
            },
        )
    };
    let clamp = |v: i32, lo: i32, hi: i32| if hi < lo { lo } else { v.clamp(lo, hi) };
    (
        clamp(x, work.x + GAP, work.right() - w - GAP),
        clamp(y, work.y + GAP, work.bottom() - h - GAP),
    )
}

/// O que o flyout lembra entre um clique e outro.
#[derive(Default)]
pub struct FlyoutState {
    /// O ícone do último clique — a âncora para reposicionar quando a altura
    /// muda.
    anchor: Mutex<Option<Area>>,
    /// Quando sumiu por perda de foco (para o clique que o fechou não reabrir).
    hidden_at: Mutex<Option<Instant>>,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|p| p.into_inner())
}

/// Cria o flyout escondido. Some ao perder o foco (como os do sistema).
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let window =
        WebviewWindowBuilder::new(app, FLYOUT_WINDOW, WebviewUrl::App("index.html".into()))
            .title("")
            .inner_size(WIDTH, 420.0)
            .decorations(false)
            .resizable(false)
            .skip_taskbar(true)
            .always_on_top(true)
            // Sombra e cantos arredondados do Windows 11 numa janela sem borda.
            .shadow(true)
            .visible(false)
            .focused(false)
            .build()?;
    let handle = app.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            if let Some(window) = handle.get_webview_window(FLYOUT_WINDOW) {
                let _ = window.hide();
            }
            *lock(&handle.state::<FlyoutState>().hidden_at) = Some(Instant::now());
        }
    });
    Ok(())
}

fn work_area_for(app: &AppHandle, icon: Area) -> Option<Area> {
    let monitor = app
        .monitor_from_point(f64::from(icon.center_x()), f64::from(icon.center_y()))
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let work = monitor.work_area();
    Some(Area {
        x: work.position.x,
        y: work.position.y,
        w: i32::try_from(work.size.width).unwrap_or(i32::MAX),
        h: i32::try_from(work.size.height).unwrap_or(i32::MAX),
    })
}

/// Posiciona o flyout junto da âncora, com o tamanho atual.
fn place(app: &AppHandle) {
    let Some(window) = app.get_webview_window(FLYOUT_WINDOW) else {
        return;
    };
    let Some(icon) = *lock(&app.state::<FlyoutState>().anchor) else {
        return;
    };
    let (Some(work), Ok(size)) = (work_area_for(app, icon), window.outer_size()) else {
        return;
    };
    let w = i32::try_from(size.width).unwrap_or(0);
    let h = i32::try_from(size.height).unwrap_or(0);
    let (x, y) = position(icon, work, w, h);
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// O clique esquerdo no ícone: abre o flyout junto dele — ou não, se foi este
/// mesmo clique que acabou de fechá-lo (tirou o foco antes de chegar aqui).
pub fn toggle(app: &AppHandle, icon: Area) {
    let Some(window) = app.get_webview_window(FLYOUT_WINDOW) else {
        return;
    };
    let state = app.state::<FlyoutState>();
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }
    if lock(&state.hidden_at).is_some_and(|t| t.elapsed() < REOPEN_GUARD) {
        return;
    }
    *lock(&state.anchor) = Some(icon);
    // Abrir o painel é o momento em que o número velho mais incomoda: uma
    // releitura aqui custa pouco e chega a tempo (como no macOS).
    crate::rotation_loop::tick(app, false);
    place(app);
    let _ = window.show();
    let _ = window.set_focus();
}

/// O front mediu o conteúdo: ajusta a altura e reposiciona na âncora (com a
/// barra embaixo, a borda de baixo fica onde estava).
pub fn fit_height(app: &AppHandle, logical_height: f64) {
    let Some(window) = app.get_webview_window(FLYOUT_WINDOW) else {
        return;
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let height = logical_height.clamp(120.0, 900.0);
    let size = PhysicalSize::new(
        (WIDTH * scale).round() as u32,
        (height * scale).round() as u32,
    );
    if window.inner_size().ok() != Some(size) {
        let _ = window.set_size(size);
        place(app);
    }
}

/// Esconde o flyout (um botão dele abriu a janela, ou o usuário escolheu sair).
pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(FLYOUT_WINDOW) {
        let _ = window.hide();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Um monitor 1920×1080 com a barra de 48 px embaixo.
    const WORK: Area = Area {
        x: 0,
        y: 0,
        w: 1920,
        h: 1032,
    };

    #[test]
    fn with_the_taskbar_at_the_bottom_it_opens_above_it_centered_on_the_icon() {
        let icon = Area {
            x: 1600,
            y: 1040,
            w: 32,
            h: 40,
        };
        assert_eq!(
            position(icon, WORK, 330, 400),
            (1616 - 165, 1032 - 400 - 12)
        );
    }

    /// Ícone perto do canto: o flyout não sai da tela.
    #[test]
    fn near_the_corner_it_stays_on_screen() {
        let icon = Area {
            x: 1880,
            y: 1040,
            w: 32,
            h: 40,
        };
        let (x, _) = position(icon, WORK, 330, 400);
        assert_eq!(x, 1920 - 330 - 12);
    }

    #[test]
    fn with_the_taskbar_at_the_top_it_opens_below_it() {
        let work = Area {
            x: 0,
            y: 48,
            w: 1920,
            h: 1032,
        };
        let icon = Area {
            x: 1600,
            y: 4,
            w: 32,
            h: 40,
        };
        assert_eq!(position(icon, work, 330, 400).1, 48 + 12);
    }

    #[test]
    fn with_the_taskbar_on_the_right_it_opens_beside_it() {
        let work = Area {
            x: 0,
            y: 0,
            w: 1860,
            h: 1080,
        };
        let icon = Area {
            x: 1870,
            y: 400,
            w: 40,
            h: 32,
        };
        let (x, y) = position(icon, work, 330, 400);
        assert_eq!(x, 1860 - 330 - 12);
        assert_eq!(y, 416 - 200, "centrado no ícone");

        // Ícone lá embaixo na barra lateral: o flyout sobe até caber.
        let low = Area { y: 1040, ..icon };
        assert_eq!(position(low, work, 330, 400).1, 1080 - 400 - 12);
    }

    /// No excedente (`^`) do Windows 11 o ícone fica DENTRO da área útil, num
    /// popup acima da barra: o flyout abre acima do clique.
    #[test]
    fn from_the_overflow_it_opens_above_the_click() {
        let icon = Area {
            x: 1500,
            y: 900,
            w: 32,
            h: 32,
        };
        assert_eq!(position(icon, WORK, 330, 400), (1516 - 165, 900 - 400 - 12));
    }

    /// Monitor secundário à esquerda (coordenadas negativas).
    #[test]
    fn works_on_a_monitor_left_of_the_primary() {
        let work = Area {
            x: -1920,
            y: 0,
            w: 1920,
            h: 1032,
        };
        let icon = Area {
            x: -400,
            y: 1040,
            w: 32,
            h: 40,
        };
        let (x, y) = position(icon, work, 330, 400);
        assert_eq!((x, y), (-384 - 165, 620));
    }
}
