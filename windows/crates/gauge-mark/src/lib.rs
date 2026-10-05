//! O anel-medidor do Falcão Token Router: o desenho da bandeja (a fração ao
//! vivo da conta que o grupo usa), ≙ o `GaugeGeometry` da barra de menus do macOS.
//!
//! O ícone do app não sai mais daqui: desde 05/10/2026 ele é da marca Falcão
//! (`app/src-tauri/icons/`). O anel mede, o falcão assina.
//!
//! Sem Tauri e sem sistema: devolve pixels (RGBA sem pré-multiplicação) e PNG, e
//! se testa por pixel.

use std::f64::consts::FRAC_PI_2;

use tiny_skia::{Color, LineCap, Paint, Path, PathBuilder, Pixmap, Stroke, Transform};

/// Quantos desenhos distintos o anel assume na bandeja (≙ `menuBarSteps`).
pub const TRAY_STEPS: u32 = 20;

/// O semáforo do painel (≙ `UsageColor`): abaixo de 66% é calmo; abaixo de 90%,
/// aviso; daí para cima, crítico. A bandeja e o painel leem os mesmos limiares —
/// com limiares separados, um diria "tranquilo" enquanto o outro já alertava.
pub const CALM_THRESHOLD: f64 = 0.66;
pub const WARNING_THRESHOLD: f64 = 0.90;

/// Verde dessaturado de propósito: o estado normal é a maior parte do tempo, e
/// um verde vivo nesse papel vira ruído.
pub const CALM: [u8; 3] = [0x5C, 0x9E, 0x73];
pub const WARNING: [u8; 3] = [0xE0, 0xB8, 0x40];
pub const CRITICAL: [u8; 3] = [0xD9, 0x52, 0x47];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Severity {
    Calm,
    Warning,
    Critical,
}

pub fn severity(fraction: f64) -> Severity {
    if fraction < CALM_THRESHOLD {
        Severity::Calm
    } else if fraction < WARNING_THRESHOLD {
        Severity::Warning
    } else {
        Severity::Critical
    }
}

/// O tema da BARRA DE TAREFAS (não o dos apps): escura pede ícone claro.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TaskbarTheme {
    Dark,
    Light,
}

fn clamp01(fraction: f64) -> f64 {
    if fraction.is_nan() {
        0.0
    } else {
        fraction.clamp(0.0, 1.0)
    }
}

/// Quanto do anel está preenchido, em graus. Satura em uma volta: um anel que
/// desse a segunda volta desenharia 110% igual a 10%.
pub fn sweep_degrees(fraction: f64) -> f64 {
    clamp01(fraction) * 360.0
}

/// Reduz a fração a um dos `steps` desenhos possíveis (≙ `GaugeGeometry.quantize`).
///
/// Para baixo, com as duas pontas protegidas: anel cheio só em 100% de verdade,
/// anel vazio só sem consumo nenhum — os dois desenhos que afirmam alguma coisa
/// ("bateu o teto", "não gastei nada"), e nenhum pode sair de arredondamento.
pub fn quantize(fraction: f64, steps: u32) -> u32 {
    let fraction = clamp01(fraction);
    if fraction <= 0.0 {
        return 0;
    }
    if fraction >= 1.0 {
        return steps;
    }
    ((fraction * f64::from(steps)) as u32).clamp(1, steps.saturating_sub(1).max(1))
}

/// A linha de centro do arco, das 12 horas no sentido HORÁRIO (em coordenadas
/// y-para-baixo, ângulo crescente), em cúbicas de até 90°. `None` sem consumo.
fn arc_path(cx: f32, cy: f32, radius: f32, fraction: f64) -> Option<Path> {
    let sweep = sweep_degrees(fraction).to_radians();
    if sweep <= 0.0 {
        return None;
    }
    let (cx, cy, r) = (f64::from(cx), f64::from(cy), f64::from(radius));
    let start = -FRAC_PI_2;
    let segments = (sweep / FRAC_PI_2).ceil().max(1.0) as usize;
    let delta = sweep / segments as f64;
    // Controle da cúbica que aproxima um arco de `delta` radianos.
    let k = 4.0 / 3.0 * (delta / 4.0).tan();
    let mut pb = PathBuilder::new();
    pb.move_to((cx + r * start.cos()) as f32, (cy + r * start.sin()) as f32);
    for i in 0..segments {
        let a0 = start + delta * i as f64;
        let a1 = a0 + delta;
        let (c0, s0, c1, s1) = (a0.cos(), a0.sin(), a1.cos(), a1.sin());
        pb.cubic_to(
            (cx + r * (c0 - k * s0)) as f32,
            (cy + r * (s0 + k * c0)) as f32,
            (cx + r * (c1 + k * s1)) as f32,
            (cy + r * (s1 - k * c1)) as f32,
            (cx + r * c1) as f32,
            (cy + r * s1) as f32,
        );
    }
    pb.finish()
}

fn paint(color: Color) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    paint
}

fn rgb(rgb: [u8; 3], alpha: f32) -> Color {
    Color::from_rgba8(rgb[0], rgb[1], rgb[2], (alpha * 255.0).round() as u8)
}

/// O anel: a trilha (o anel a 100%, esmaecido — mostra o que ainda cabe) e, por
/// cima, o arco com ponta redonda.
#[allow(clippy::too_many_arguments)]
fn draw_ring(
    pixmap: &mut Pixmap,
    cx: f32,
    cy: f32,
    radius: f32,
    width: f32,
    fraction: Option<f64>,
    ring: Color,
    track: Option<Color>,
) {
    if let (Some(track), Some(circle)) = (track, PathBuilder::from_circle(cx, cy, radius)) {
        let stroke = Stroke {
            width,
            ..Stroke::default()
        };
        pixmap.stroke_path(&circle, &paint(track), &stroke, Transform::identity(), None);
    }
    if let Some(arc) = fraction.and_then(|f| arc_path(cx, cy, radius, f)) {
        let stroke = Stroke {
            width,
            line_cap: LineCap::Round,
            ..Stroke::default()
        };
        pixmap.stroke_path(&arc, &paint(ring), &stroke, Transform::identity(), None);
    }
}

/// Pixels sem pré-multiplicação, na ordem RGBA — o que a bandeja e os PNGs
/// esperam.
fn straight_rgba(pixmap: &Pixmap) -> Vec<u8> {
    pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}

// MARK: - Bandeja

/// O que distingue um desenho da bandeja de outro — a chave do cache do app
/// (no máximo 21 passos × 3 cores × 2 temas por tamanho, na vida do processo).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TrayKey {
    /// O passo quantizado; `None` = conta sem amostra (anel vazio, "pronta").
    pub step: Option<u32>,
    /// A cor vem da fração REAL, não da quantizada: 66% já é aviso, como no
    /// painel, mesmo caindo no passo de 65%.
    pub severity: Severity,
    pub theme: TaskbarTheme,
    pub size: u32,
}

impl TrayKey {
    pub fn new(fraction: Option<f64>, theme: TaskbarTheme, size: u32) -> Self {
        TrayKey {
            step: fraction.map(|f| quantize(f, TRAY_STEPS)),
            severity: fraction.map_or(Severity::Calm, severity),
            theme,
            size,
        }
    }
}

/// Um ícone da bandeja pronto: quadrado de `size` px, RGBA sem pré-multiplicação.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TrayIcon {
    pub size: u32,
    pub rgba: Vec<u8>,
}

/// A cor do anel. No estado calmo é a do tema da barra — branco na escura,
/// quase preto na clara —, não verde: o ícone fica visível o tempo todo, e cor
/// permanente ali compete com o resto da bandeja (≙ `UsageColor.menuBar`). A cor
/// só entra quando há o que dizer.
fn tray_color(severity: Severity, theme: TaskbarTheme) -> [u8; 3] {
    match (severity, theme) {
        (Severity::Calm, TaskbarTheme::Dark) => [0xFF, 0xFF, 0xFF],
        (Severity::Calm, TaskbarTheme::Light) => [0x1F, 0x1F, 0x1F],
        (Severity::Warning, _) => WARNING,
        (Severity::Critical, _) => CRITICAL,
    }
}

pub fn render_tray(key: TrayKey) -> TrayIcon {
    let size = key.size.max(1);
    let Some(mut pixmap) = Pixmap::new(size, size) else {
        return TrayIcon {
            size,
            rgba: vec![0; (size * size * 4) as usize],
        };
    };
    let side = size as f32;
    // A 16 px (100% de escala) o traço fica em ~2 px: menos que isso e o anel
    // deixa de ler como anel.
    let width = (side * 0.14).max(2.0);
    let margin = side * 0.03;
    let radius = (side - 2.0 * margin - width) / 2.0;
    let color = tray_color(key.severity, key.theme);
    let fraction = key
        .step
        .map(|s| f64::from(s) / f64::from(TRAY_STEPS))
        .filter(|f| *f > 0.0);
    draw_ring(
        &mut pixmap,
        side / 2.0,
        side / 2.0,
        radius,
        width,
        fraction,
        rgb(color, 1.0),
        Some(rgb(color, 0.3)),
    );
    TrayIcon {
        size,
        rgba: straight_rgba(&pixmap),
    }
}

/// O ícone da bandeja para uma fração (`None` = sem amostra).
pub fn tray_icon(fraction: Option<f64>, theme: TaskbarTheme, size: u32) -> TrayIcon {
    render_tray(TrayKey::new(fraction, theme, size))
}

/// O mesmo desenho em PNG — para a prévia do `icongen --tray` (conferir a olho
/// os estados que os testes conferem por pixel).
pub fn tray_png(key: TrayKey) -> Option<Vec<u8>> {
    let icon = render_tray(key);
    let mut pixmap = Pixmap::new(icon.size, icon.size)?;
    let (pixels, _) = icon.rgba.as_chunks::<4>();
    for (dst, [r, g, b, a]) in pixmap.pixels_mut().iter_mut().zip(pixels) {
        *dst = tiny_skia::ColorU8::from_rgba(*r, *g, *b, *a).premultiply();
    }
    pixmap.encode_png().ok()
}
