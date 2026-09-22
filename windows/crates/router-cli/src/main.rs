//! A CLI `router` que o app empacota e que faz a ponte com o terminal.
//!
//! Nesta fatia (o sensor) só o `statusline` está implementado; `launch`,
//! `is-group`, `rotate`, `doctor` e `measure` entram na Fase 4. Sem argumento, o
//! comando é `statusline` — igual ao macOS.

mod statusline;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("statusline");

    match command {
        "statusline" => statusline::run(),
        _ => {
            eprintln!(
                "uso: router [statusline|launch <grupo>|is-group <nome>|rotate|doctor|measure [grupo]]"
            );
            std::process::exit(2);
        }
    }
}
