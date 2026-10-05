//! Prévia da bandeja: o anel nos estados que importam, nos dois temas e em 16/32 px.
//!
//! Uso: `cargo run -p gauge-mark --bin icongen -- <pasta de saída>`
//!
//! Os ícones do app não saem mais daqui — desde 05/10/2026 são da marca Falcão,
//! em `app/src-tauri/icons/`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use gauge_mark::{tray_png, TaskbarTheme, TrayKey};

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
    // `--tray` era o modo da prévia quando o binário também gerava o ícone do app;
    // segue aceito para não quebrar quem o tem no histórico do terminal.
    let args: Vec<_> = std::env::args_os()
        .skip(1)
        .filter(|a| a != "--tray")
        .collect();
    let Some(out) = args.first().map(PathBuf::from) else {
        eprintln!("uso: icongen <pasta de saída>");
        return ExitCode::from(2);
    };
    if let Err(e) = fs::create_dir_all(&out) {
        eprintln!("icongen: {}: {e}", out.display());
        return ExitCode::FAILURE;
    }
    tray_preview(&out)
}
