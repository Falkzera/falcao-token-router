//! A marca do app — o anel-medidor — testada por pixel: a bandeja do Windows e
//! o ícone do app saem do mesmo desenho (≙ `GaugeGeometry` + `icon.swift` do
//! macOS), e o que o anel afirma tem de estar nos pixels.

use gauge_mark::{
    app_icon, quantize, severity, tray_icon, Severity, TaskbarTheme, CRITICAL, TRAY_STEPS, WARNING,
};

/// O pixel (x, y) de uma imagem RGBA sem pré-multiplicação.
fn px(rgba: &[u8], side: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * side + x) * 4) as usize;
    [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
}

// MARK: - Quantização (≙ GaugeGeometry.quantize)

/// Anel cheio só em 100% de verdade, anel vazio só sem consumo nenhum: são os
/// dois desenhos que afirmam alguma coisa, e nenhum pode sair de arredondamento.
#[test]
fn the_ends_of_the_ring_are_never_reached_by_rounding() {
    assert_eq!(quantize(0.0, 20), 0);
    assert_eq!(quantize(-0.3, 20), 0);
    assert_eq!(quantize(0.001, 20), 1, "consumo mínimo não é anel vazio");
    assert_eq!(quantize(0.999, 20), 19, "quase cheio não é cheio");
    assert_eq!(quantize(1.0, 20), 20);
    assert_eq!(quantize(1.7, 20), 20, "satura em uma volta");
}

#[test]
fn quantizing_rounds_down_between_the_ends() {
    assert_eq!(quantize(0.5, 20), 10);
    assert_eq!(quantize(0.549, 20), 10);
    assert_eq!(quantize(0.55, 20), 11);
}

// MARK: - Semáforo (≙ UsageColor)

/// Os mesmos limiares do painel: com limiares separados, um diria "tranquilo"
/// enquanto o outro já alertava. 66% é aviso; 90% é crítico.
#[test]
fn severity_uses_the_panel_thresholds() {
    assert_eq!(severity(0.0), Severity::Calm);
    assert_eq!(severity(0.659), Severity::Calm);
    assert_eq!(severity(0.66), Severity::Warning);
    assert_eq!(severity(0.899), Severity::Warning);
    assert_eq!(severity(0.90), Severity::Critical);
    assert_eq!(severity(1.2), Severity::Critical);
}

// MARK: - Bandeja

const SIDE: u32 = 32;

/// Onde fica o anel num ícone de 32 px: o centro do traço a 12, 3, 6 e 9 horas.
fn at_12(rgba: &[u8]) -> [u8; 4] {
    px(rgba, SIDE, SIDE / 2, 2)
}
fn at_3(rgba: &[u8]) -> [u8; 4] {
    px(rgba, SIDE, SIDE - 3, SIDE / 2)
}
fn at_6(rgba: &[u8]) -> [u8; 4] {
    px(rgba, SIDE, SIDE / 2, SIDE - 3)
}
fn at_9(rgba: &[u8]) -> [u8; 4] {
    px(rgba, SIDE, 2, SIDE / 2)
}

/// A trilha é o anel a 100%, esmaecido: mostra o que ainda cabe.
fn is_track(p: [u8; 4]) -> bool {
    (50..=110).contains(&p[3])
}
fn is_arc(p: [u8; 4]) -> bool {
    p[3] >= 200
}

#[test]
fn the_tray_icon_has_the_requested_size() {
    let icon = tray_icon(Some(0.5), TaskbarTheme::Dark, SIDE);
    assert_eq!(icon.size, SIDE);
    assert_eq!(icon.rgba.len(), (SIDE * SIDE * 4) as usize);
}

/// O arco começa às 12 e gira no sentido HORÁRIO: metade cheia pinta a direita
/// (3 horas) e deixa a esquerda (9 horas) só com a trilha.
#[test]
fn half_usage_fills_the_right_half_clockwise() {
    let icon = tray_icon(Some(0.5), TaskbarTheme::Dark, SIDE);
    assert!(is_arc(at_3(&icon.rgba)), "3 horas: {:?}", at_3(&icon.rgba));
    assert!(
        is_track(at_9(&icon.rgba)),
        "9 horas: {:?}",
        at_9(&icon.rgba)
    );
    // O centro do ícone é vazio.
    assert_eq!(px(&icon.rgba, SIDE, SIDE / 2, SIDE / 2)[3], 0);
}

#[test]
fn a_full_ring_is_opaque_all_around() {
    let icon = tray_icon(Some(1.0), TaskbarTheme::Dark, SIDE);
    for p in [
        at_12(&icon.rgba),
        at_3(&icon.rgba),
        at_6(&icon.rgba),
        at_9(&icon.rgba),
    ] {
        assert!(is_arc(p), "{p:?}");
    }
}

/// Conta sem amostra ("pronta") e consumo zero: só a trilha, nada de arco.
#[test]
fn no_sample_draws_only_the_track() {
    for icon in [
        tray_icon(None, TaskbarTheme::Dark, SIDE),
        tray_icon(Some(0.0), TaskbarTheme::Dark, SIDE),
    ] {
        for p in [
            at_12(&icon.rgba),
            at_3(&icon.rgba),
            at_6(&icon.rgba),
            at_9(&icon.rgba),
        ] {
            assert!(is_track(p), "{p:?}");
        }
    }
}

/// No estado calmo o anel tem a cor do tema da barra de tarefas — branco na
/// escura, quase preto na clara —, não verde: um ícone que fica visível o tempo
/// todo não deve competir com o resto da bandeja. A cor só entra quando há o
/// que dizer.
#[test]
fn calm_follows_the_taskbar_theme() {
    let dark = at_3(&tray_icon(Some(0.5), TaskbarTheme::Dark, SIDE).rgba);
    let light = at_3(&tray_icon(Some(0.5), TaskbarTheme::Light, SIDE).rgba);
    assert!(dark[0] > 230 && dark[1] > 230 && dark[2] > 230, "{dark:?}");
    assert!(light[0] < 60 && light[1] < 60 && light[2] < 60, "{light:?}");
}

fn close_to(p: [u8; 4], rgb: [u8; 3]) -> bool {
    p.iter()
        .zip(rgb)
        .all(|(a, b)| (i16::from(*a) - i16::from(b)).abs() <= 6)
}

#[test]
fn warning_and_critical_have_their_colors_in_both_themes() {
    for theme in [TaskbarTheme::Dark, TaskbarTheme::Light] {
        let warn = at_3(&tray_icon(Some(0.7), theme, SIDE).rgba);
        assert!(close_to(warn, WARNING), "{warn:?}");
        let crit = at_3(&tray_icon(Some(0.95), theme, SIDE).rgba);
        assert!(close_to(crit, CRITICAL), "{crit:?}");
    }
}

/// O desenho é o da fração QUANTIZADA: 0,52 e 0,54 caem no mesmo passo e têm
/// de dar o mesmo bitmap (a bandeja guarda um por passo).
#[test]
fn fractions_in_the_same_step_draw_the_same_icon() {
    let a = tray_icon(Some(0.52), TaskbarTheme::Dark, SIDE);
    let b = tray_icon(Some(0.54), TaskbarTheme::Dark, SIDE);
    assert_eq!(quantize(0.52, TRAY_STEPS), quantize(0.54, TRAY_STEPS));
    assert_eq!(a.rgba, b.rgba);
}

/// A 16 px (100% de escala) o traço não afina a ponto de sumir.
#[test]
fn the_ring_survives_at_16_px() {
    let icon = tray_icon(Some(1.0), TaskbarTheme::Dark, 16);
    // Um traço de ~2 px raramente cobre um pixel inteiro: conta a cobertura
    // de pelo menos metade.
    let covered = icon.rgba.chunks(4).filter(|p| p[3] >= 128).count();
    assert!(covered >= 40, "só {covered} pixels cobertos");
}

// MARK: - Ícone do app

/// A placa (superelipse verde) com o anel branco a 62% — o arco mais cheio que
/// ainda cabe no verde; um ícone permanentemente âmbar seria alarme falso.
#[test]
fn the_app_icon_is_a_green_plate_with_a_white_ring() {
    let side = 256;
    let icon = app_icon(side);
    assert_eq!(icon.len(), (side * side * 4) as usize);

    // Canto: fora da placa, transparente.
    assert_eq!(px(&icon, side, 1, 1)[3], 0);
    // Centro: dentro da placa, no furo do anel — verde, opaco.
    let center = px(&icon, side, side / 2, side / 2);
    assert_eq!(center[3], 255);
    assert!(center[1] > center[0] && center[1] > center[2], "{center:?}");
    // Arco a 3 horas (dentro dos 62%), na linha de centro do traço: branco.
    let ring_right = px(&icon, side, side / 2 + side * 20 / 100, side / 2);
    assert!(
        ring_right[0] > 225 && ring_right[1] > 225 && ring_right[2] > 225,
        "{ring_right:?}"
    );
}
