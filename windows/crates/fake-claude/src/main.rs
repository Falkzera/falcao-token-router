//! Um `claude` de mentira, só para os testes de integração da CLI `router`.
//! Nunca é empacotado nem roda fora dos testes (a CLI o acha por
//! `ROUTER_CLAUDE_BIN`, que só os testes setam).
//!
//! Controlado por variáveis de ambiente:
//! - `FAKE_CLAUDE_RECORD`: arquivo onde anexa uma linha JSON por execução — os
//!   argumentos, a pasta de trabalho, o perfil (`CLAUDE_CONFIG_DIR`) e quais
//!   variáveis sensíveis chegaram (nomes, nunca valores);
//! - `FAKE_CLAUDE_EXIT`: o código de saída (padrão 0);
//! - `FAKE_CLAUDE_SLEEP_MS`: espera antes de responder (prazo da sonda);
//! - `FAKE_CLAUDE_USAGE`: o que o `/usage` imprime num perfil logado.
//!
//! No `/usage`, "logado" = o perfil tem `.credentials.json` — como o real, que
//! num perfil sem login sai com código 0 e só o resumo do `--print`.

use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

/// Variáveis que o `router` precisa remover (ou manter) — o registro diz quais
/// chegaram. O ambiente do Windows não tem caixa: `var` acha em qualquer caixa.
const WATCHED: &[&str] = &[
    "HTTPS_PROXY",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_BASE_URL",
    "CLAUDE_SECURESTORAGE_CONFIG_DIR",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDECODE",
    "CLAUDE_CODE_ENTRYPOINT",
    "DISABLE_TELEMETRY",
    "ANTHROPIC_MODEL",
];

const DEFAULT_USAGE: &str = "You are currently using your subscription to power your Claude Code usage\r\n\r\nCurrent session: 12% used \u{b7} resets Sep 22, 8:40pm (America/Sao_Paulo)\r\nCurrent week (all models): 34% used \u{b7} resets Sep 23, 4am (America/Sao_Paulo)\r\nCurrent week (Fable): 56% used \u{b7} resets Sep 23, 4am (America/Sao_Paulo)\r\n";

const LOGGED_OUT: &str = "Total cost:            $0.0000\r\nTotal duration (API):  0s\r\nTotal duration (wall): 0s\r\nTotal code changes:    0 lines added, 0 lines removed\r\nUsage:                 0 input, 0 output, 0 cache read, 0 cache write\r\n";

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let profile = env::var("CLAUDE_CONFIG_DIR").ok();
    record(&args, profile.as_deref());

    if let Some(ms) = env::var("FAKE_CLAUDE_SLEEP_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
    {
        thread::sleep(Duration::from_millis(ms));
    }

    if args.iter().any(|a| a == "/usage") {
        let dir = profile.map(PathBuf::from).unwrap_or_else(|| {
            PathBuf::from(env::var("USERPROFILE").unwrap_or_default()).join(".claude")
        });
        let text = if dir.join(".credentials.json").is_file() {
            env::var("FAKE_CLAUDE_USAGE").unwrap_or_else(|_| DEFAULT_USAGE.to_string())
        } else {
            LOGGED_OUT.to_string()
        };
        print!("{text}");
    } else if args.first().map(String::as_str) == Some("--version") {
        println!("2.1.280 (Claude Code)");
    }
    let _ = std::io::stdout().flush();

    let code = env::var("FAKE_CLAUDE_EXIT")
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(0);
    std::process::exit(code);
}

fn record(args: &[String], profile: Option<&str>) {
    let Ok(path) = env::var("FAKE_CLAUDE_RECORD") else {
        return;
    };
    let present: Vec<&str> = WATCHED
        .iter()
        .copied()
        .filter(|k| env::var_os(k).is_some())
        .collect();
    let line = serde_json::json!({
        "args": args,
        "cwd": env::current_dir().map(|p| p.display().to_string()).unwrap_or_default(),
        "claudeConfigDir": profile,
        "present": present,
    });
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}
