//! A marca do Falcão Token Router: o anel-medidor.
//!
//! Um desenho só para a bandeja (a fração ao vivo da conta que o grupo usa) e
//! para o ícone do app (congelado em 62%) — como no macOS, onde o
//! `GaugeGeometry` do core é compilado também pelo gerador do ícone: sem isso,
//! "identidade visual" viraria dois desenhos parecidos que alguém tem de lembrar
//! de manter em sincronia.
//!
//! Sem Tauri e sem sistema: devolve pixels (RGBA sem pré-multiplicação) e PNG, e
//! se testa por pixel.

use std::f64::consts::FRAC_PI_2;

use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LinearGradient, Mask, Paint, Path, PathBuilder, Pixmap,
    Point, Rect, SpreadMode, Stroke, Transform,
};

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

/// A fração que o ícone do app mostra. 62%, e não um número redondo, porque é o
/// arco mais cheio que ainda cabe no verde: um ícone permanentemente âmbar seria
/// alarme falso.
pub const ICON_FRACTION: f64 = 0.62;

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

// MARK: - Ícone do app (≙ Scripts/icon.swift)

const MARK_TOP: [u8; 3] = [107, 179, 133]; // (0.42, 0.70, 0.52)
const MARK_BOTTOM: [u8; 3] = [51, 102, 77]; // (0.20, 0.40, 0.30)

/// Superelipse — o canto contínuo, em vez de um arco de círculo colado numa
/// reta (um retângulo arredondado comum dá a silhueta errada).
fn squircle(rect: Rect, exponent: f64) -> Option<Path> {
    let (a, b) = (
        f64::from(rect.width()) / 2.0,
        f64::from(rect.height()) / 2.0,
    );
    let (cx, cy) = (f64::from(rect.x()) + a, f64::from(rect.y()) + b);
    let samples = 720;
    let mut pb = PathBuilder::new();
    for step in 0..=samples {
        let t = f64::from(step) / f64::from(samples) * std::f64::consts::TAU;
        let (cos, sin) = (t.cos(), t.sin());
        let x = cx + a * cos.abs().powf(2.0 / exponent).copysign(cos);
        let y = cy + b * sin.abs().powf(2.0 / exponent).copysign(sin);
        if step == 0 {
            pb.move_to(x as f32, y as f32);
        } else {
            pb.line_to(x as f32, y as f32);
        }
    }
    pb.close();
    pb.finish()
}

fn vertical_gradient(
    top: f32,
    bottom: f32,
    from: Color,
    to: Color,
) -> Option<tiny_skia::Shader<'static>> {
    LinearGradient::new(
        Point::from_xy(0.0, top),
        Point::from_xy(0.0, bottom),
        vec![GradientStop::new(0.0, from), GradientStop::new(1.0, to)],
        SpreadMode::Pad,
        Transform::identity(),
    )
}

fn draw_app_icon(side: u32) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new(side, side)?;
    let s = side as f32;
    // Abaixo de 64 px o desenho engrossa em vez de encolher junto: as proporções
    // do tamanho grande dariam, a 16 px, um anel que o antialiasing dissolve.
    // Brilho e fio de luz também saem — nessa escala viram sujeira.
    let compact = side < 64;
    let inset = s * if compact { 0.03 } else { 0.06 };
    let plate = Rect::from_xywh(inset, inset, s - inset * 2.0, s - inset * 2.0)?;
    let shape = squircle(plate, 5.0)?;

    let fill = Paint {
        anti_alias: true,
        shader: vertical_gradient(
            plate.top(),
            plate.bottom(),
            rgb(MARK_TOP, 1.0),
            rgb(MARK_BOTTOM, 1.0),
        )?,
        ..Paint::default()
    };
    pixmap.fill_path(
        &shape,
        &fill,
        FillRule::Winding,
        Transform::identity(),
        None,
    );

    if !compact {
        let mut clip = Mask::new(side, side)?;
        clip.fill_path(&shape, FillRule::Winding, true, Transform::identity());
        // Brilho: a metade de cima da placa, esmaecendo para baixo.
        let gloss_rect =
            Rect::from_xywh(plate.x(), plate.y(), plate.width(), plate.height() * 0.58)?;
        let gloss = Paint {
            anti_alias: true,
            shader: vertical_gradient(
                gloss_rect.top(),
                gloss_rect.bottom(),
                rgb([255, 255, 255], 0.34),
                rgb([255, 255, 255], 0.0),
            )?,
            ..Paint::default()
        };
        pixmap.fill_rect(gloss_rect, &gloss, Transform::identity(), Some(&clip));
        // Fio de luz na borda: o realce que separa a placa do fundo (recortado
        // pela placa, só a metade de dentro do traço aparece — como no macOS).
        let edge = Stroke {
            width: s * 0.006,
            ..Stroke::default()
        };
        pixmap.stroke_path(
            &shape,
            &paint(rgb([255, 255, 255], 0.45)),
            &edge,
            Transform::identity(),
            Some(&clip),
        );
    }

    let ring_side = plate.width() * if compact { 0.68 } else { 0.54 };
    let width =
        (ring_side * if compact { 0.24 } else { 0.155 }).max(if compact { 2.0 } else { 0.0 });
    draw_ring(
        &mut pixmap,
        plate.x() + plate.width() / 2.0,
        plate.y() + plate.height() / 2.0,
        (ring_side - width) / 2.0,
        width,
        Some(ICON_FRACTION),
        rgb([255, 255, 255], 0.96),
        Some(rgb([255, 255, 255], 0.26)),
    );
    Some(pixmap)
}

/// O ícone do app num quadrado de `side` px, RGBA sem pré-multiplicação.
pub fn app_icon(side: u32) -> Vec<u8> {
    draw_app_icon(side)
        .map(|p| straight_rgba(&p))
        .unwrap_or_else(|| vec![0; (side * side * 4) as usize])
}

/// O mesmo ícone em PNG.
pub fn app_icon_png(side: u32) -> Option<Vec<u8>> {
    draw_app_icon(side)?.encode_png().ok()
}

/// Um `.ico` com cada tamanho desenhado no próprio tamanho (os pequenos no
/// traço grosso do modo compacto) — redimensionar o de 1024 px daria, a 16 px,
/// o anel borrado que o compacto existe para evitar. Entradas em PNG, que o
/// Windows lê desde o Vista.
pub fn app_icon_ico(sizes: &[u32]) -> Option<Vec<u8>> {
    let images: Vec<(u32, Vec<u8>)> = sizes
        .iter()
        .map(|&s| app_icon_png(s).map(|png| (s, png)))
        .collect::<Option<_>>()?;
    let count = u16::try_from(images.len()).ok()?;
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes()); // reservado
    out.extend_from_slice(&1u16.to_le_bytes()); // tipo: ícone
    out.extend_from_slice(&count.to_le_bytes());
    let mut offset = 6 + 16 * u32::from(count);
    for (side, png) in &images {
        // 256 se escreve 0 no diretório (o campo tem um byte).
        let dim = if *side >= 256 { 0 } else { *side as u8 };
        out.extend_from_slice(&[dim, dim, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes()); // planos
        out.extend_from_slice(&32u16.to_le_bytes()); // bits por pixel
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for (_, png) in images {
        out.extend_from_slice(&png);
    }
    Some(out)
}
