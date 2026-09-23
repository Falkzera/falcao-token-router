//! O tamanho da janela Grupos/Ajustes (a `home`): abre em 520×620 (≙ macOS),
//! redimensiona e maximiza como qualquer janela, e reabre no último tamanho —
//! guardado no `settings.json` do app (pedido de 23/09/2026; antes o tamanho
//! era fixo).
//!
//! Tudo em pixels LÓGICOS: o mesmo tamanho aparente em qualquer escala de
//! monitor (o físico mudaria ao trocar de monitor ou de escala). A POSIÇÃO não
//! é guardada: a janela abre centralizada na área útil do monitor principal e
//! encolhida para caber nela, com folga (`prevent_overflow_with_margin` do
//! Tauri, que conta a barra de título) — um tamanho guardado num monitor maior
//! nunca abre para fora da tela.

use serde::{Deserialize, Serialize};
use tauri::{PhysicalSize, WebviewWindow};

use crate::settings::SettingsStore;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSize {
    pub width: f64,
    pub height: f64,
    /// Estava maximizada. `width`/`height` seguem sendo o tamanho para onde
    /// ela volta ao sair do maximizado.
    #[serde(default)]
    pub maximized: bool,
}

/// O tamanho de sempre (≙ o 520×620 do macOS): o da primeira abertura.
pub const DEFAULT: WindowSize = WindowSize {
    width: 520.0,
    height: 620.0,
    maximized: false,
};

/// A largura mínima é a de sempre — o layout foi desenhado nela. A altura pode
/// ser menor: as páginas rolam (e os diálogos também).
pub const MIN_WIDTH: f64 = 520.0;
pub const MIN_HEIGHT: f64 = 420.0;

/// A folga em volta de um tamanho guardado que não cabe na área útil (de um
/// monitor maior): encolhida até ENCOSTAR na área, ela pareceria maximizada
/// sem estar — e a conta do Tauri deixa de fora a borda visível de 1 px (foi
/// o que se viu em 23/09/2026: a moldura passava 1 px de cada lado).
pub const SCREEN_MARGIN: f64 = 32.0;

/// Um teto que nenhum monitor passa (o 8K tem 7680 px): a conta do Tauri para
/// caber na tela trabalha em `u32` físico, e um número absurdo no arquivo
/// transbordaria lá.
const MAX_SIDE: f64 = 16384.0;

impl WindowSize {
    /// O tamanho com que a janela abre: o guardado (ou o padrão), nunca abaixo
    /// do mínimo. O teto de verdade é a tela, e esse o Tauri aplica na criação.
    pub fn opening(saved: Option<WindowSize>) -> WindowSize {
        let Some(saved) = saved else {
            return DEFAULT;
        };
        let side = |value: f64, min: f64, default: f64| {
            if value.is_finite() {
                value.clamp(min, MAX_SIDE)
            } else {
                default
            }
        };
        WindowSize {
            width: side(saved.width, MIN_WIDTH, DEFAULT.width),
            height: side(saved.height, MIN_HEIGHT, DEFAULT.height),
            maximized: saved.maximized,
        }
    }

    /// O que um redimensionamento ensina: maximizada, só isso — o tamanho para
    /// onde ela volta continua o de antes; no tamanho normal, o novo vale.
    pub fn after_resize(self, width: f64, height: f64, maximized: bool) -> WindowSize {
        if maximized {
            WindowSize {
                maximized: true,
                ..self
            }
        } else {
            WindowSize {
                width,
                height,
                maximized: false,
            }
        }
    }
}

/// Acompanha o tamanho a cada `Resized`, só na memória: o evento chega a cada
/// passo do arrasto, e o arquivo é gravado ao fechar a janela e na saída do app.
pub fn track(window: &WebviewWindow, store: &SettingsStore, size: PhysicalSize<u32>) {
    // Minimizada, a janela "mede" 0×0 — não é um tamanho.
    if size.width == 0 || size.height == 0 || window.is_minimized().unwrap_or(false) {
        return;
    }
    let maximized = window.is_maximized().unwrap_or(false);
    let logical = size.to_logical::<f64>(window.scale_factor().unwrap_or(1.0));
    store.remember(|s| {
        let last = s.home_window.unwrap_or(DEFAULT);
        s.home_window = Some(last.after_resize(logical.width, logical.height, maximized));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(width: f64, height: f64, maximized: bool) -> WindowSize {
        WindowSize {
            width,
            height,
            maximized,
        }
    }

    #[test]
    fn the_first_opening_is_the_size_it_always_had() {
        assert_eq!(WindowSize::opening(None), size(520.0, 620.0, false));
    }

    #[test]
    fn it_reopens_at_the_last_size_maximized_or_not() {
        assert_eq!(
            WindowSize::opening(Some(size(900.0, 700.0, false))),
            size(900.0, 700.0, false)
        );
        assert_eq!(
            WindowSize::opening(Some(size(900.0, 700.0, true))),
            size(900.0, 700.0, true)
        );
    }

    /// O arquivo é só nosso, mas pode ter sido editado à mão: pequeno demais
    /// sobe ao mínimo, absurdo desce a um teto que nenhum monitor passa (a
    /// conta do Tauri para caber na tela trabalha em `u32` físico).
    #[test]
    fn a_saved_size_never_goes_below_the_minimum_nor_to_absurd_sizes() {
        assert_eq!(
            WindowSize::opening(Some(size(100.0, 100.0, false))),
            size(MIN_WIDTH, MIN_HEIGHT, false)
        );
        let huge = WindowSize::opening(Some(size(1e12, -5.0, false)));
        assert!(huge.width <= 16384.0, "{huge:?}");
        assert_eq!(huge.height, MIN_HEIGHT);
    }

    /// Maximizar não é um tamanho novo: é para o de antes que ela volta.
    #[test]
    fn maximizing_keeps_the_size_to_come_back_to() {
        let maximized = DEFAULT.after_resize(1920.0, 1040.0, true);
        assert_eq!(maximized, size(520.0, 620.0, true));
        assert_eq!(
            maximized.after_resize(800.0, 700.0, false),
            size(800.0, 700.0, false)
        );
        assert_eq!(
            size(640.0, 480.0, false).after_resize(700.0, 500.0, false),
            size(700.0, 500.0, false)
        );
    }
}
