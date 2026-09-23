//! Gera os ícones do app a partir do anel (≙ `Scripts/icon.swift`).
//!
//! Uso: `cargo run -p gauge-mark --bin icongen -- <pasta de saída>`
//!      `cargo run -p gauge-mark --bin icongen -- --tray <pasta>` (prévia da bandeja)
//!
//! Escreve o que o `tauri.conf.json` lista: `icon.ico` (cada tamanho desenhado
//! no próprio tamanho — é o que o `tauri-build` embute no .exe), `32x32.png`,
//! `128x128.png`, `128x128@2x.png` e `icon.png`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use gauge_mark::{app_icon_ico, app_icon_png, tray_png, TaskbarTheme, TrayKey};

/// A bandeja nos estados que importam, nos dois temas e em 16/32 px.
fn tray_preview(out: &Path) -> ExitCode {
    let states: [(&str, Option<f64>); 6] = [
        ("pronta", None),
        ("0", Some(0.0)),
        ("35", Some(0.35)),
        ("70", Some(0.70)),
        ("95", Some(0.95)),
        ("100", Some(1.0)),
    ];
    for (theme_name, theme) in [
        ("escura", TaskbarTheme::Dark),
        ("clara", TaskbarTheme::Light),
    ] {
        for size in [16, 32] {
            for (state, fraction) in states {
                let name = format!("bandeja-{theme_name}-{size}-{state}.png");
                let Some(png) = tray_png(TrayKey::new(fraction, theme, size)) else {
                    eprintln!("icongen: não consegui desenhar {name}");
                    return ExitCode::FAILURE;
                };
                if let Err(e) = fs::write(out.join(&name), png) {
                    eprintln!("icongen: {name}: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
    }
    println!("prévia da bandeja em {}", out.display());
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let tray = args.first().is_some_and(|a| a == "--tray");
    let Some(out) = args.get(usize::from(tray)).map(PathBuf::from) else {
        eprintln!("uso: icongen [--tray] <pasta de saída>");
        return ExitCode::from(2);
    };
    if let Err(e) = fs::create_dir_all(&out) {
        eprintln!("icongen: {}: {e}", out.display());
        return ExitCode::FAILURE;
    }
    if tray {
        return tray_preview(&out);
    }
    let pngs = [
        ("32x32.png", 32),
        ("128x128.png", 128),
        ("128x128@2x.png", 256),
        ("icon.png", 512),
    ];
    for (name, side) in pngs {
        let Some(bytes) = app_icon_png(side) else {
            eprintln!("icongen: não consegui desenhar {name}");
            return ExitCode::FAILURE;
        };
        if let Err(e) = fs::write(out.join(name), bytes) {
            eprintln!("icongen: {name}: {e}");
            return ExitCode::FAILURE;
        }
    }
    let Some(ico) = app_icon_ico(&[16, 20, 24, 32, 40, 48, 64, 128, 256]) else {
        eprintln!("icongen: não consegui montar o icon.ico");
        return ExitCode::FAILURE;
    };
    if let Err(e) = fs::write(out.join("icon.ico"), ico) {
        eprintln!("icongen: icon.ico: {e}");
        return ExitCode::FAILURE;
    }
    println!("ícones em {}", out.display());
    ExitCode::SUCCESS
}
