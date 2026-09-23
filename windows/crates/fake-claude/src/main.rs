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
//! - `FAKE_CLAUDE_USAGE`: o que o `/usage` imprime num perfil logado;
//! - `FAKE_CLAUDE_LOGIN`: o desfecho do `auth login` — `ok:<e-mail>` (grava a
//!   credencial e a identidade no perfil e diz "Login successful."),
//!   `code:<e-mail>` (o mesmo, depois de ler um código `a#b` do stdin; código
//!   sem `#` dá "Invalid code"), `quiet:<e-mail>` (grava e sai com 0 sem dizer
//!   nada), `nodisk` (diz que deu certo e não grava), `fail:<motivo>` (sai
//!   com 1) ou `hang` (espera até ser encerrado).
//!
//! No `/usage`, "logado" = o perfil tem `.credentials.json` — como o real, que
//! num perfil sem login sai com código 0 e só o resumo do `--print`.
//!
//! `statusline-echo` faz o papel de uma status line do usuário (o modo "meu
//! comando" do router): lê o JSON do stdin ATÉ O FIM (só termina com EOF) e
//! imprime `eco: <modelo> encadeado=<ROUTER_STATUSLINE_CHAINED>`.
//!
//! O `auth login` imprime o que o 2.1.280 imprime (lido no JS do binário em
//! 23/09/2026), com o link num hyperlink OSC 8 quando a saída é terminal.

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, IsTerminal, Write};
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

    if args.first().map(String::as_str) == Some("statusline-echo") {
        let mut input = String::new();
        let _ = std::io::Read::read_to_string(&mut std::io::stdin(), &mut input);
        let json: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
        let model = json["model"]["display_name"].as_str().unwrap_or("?");
        let chained = env::var("ROUTER_STATUSLINE_CHAINED").unwrap_or_default();
        println!("eco: {model} encadeado={chained}");
    } else if args.first().map(String::as_str) == Some("auth")
        && args.get(1).map(String::as_str) == Some("login")
    {
        let email = args
            .iter()
            .position(|a| a == "--email")
            .and_then(|i| args.get(i + 1))
            .cloned();
        std::process::exit(auth_login(profile.as_deref(), email.as_deref()));
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

/// O `claude auth login` de mentira. Devolve o código de saída.
fn auth_login(profile: Option<&str>, email_hint: Option<&str>) -> i32 {
    let mut url =
        "https://claude.com/cai/oauth/authorize?code=true&client_id=fake&state=fake".to_string();
    if let Some(email) = email_hint {
        url.push_str("&login_hint=");
        url.push_str(email);
    }
    let link = if std::io::stdout().is_terminal() {
        format!("\x1b]8;;{url}\x07\x1b[94m{url}\x1b[39m\x1b]8;;\x07")
    } else {
        url
    };
    let mut out = std::io::stdout();
    let _ = write!(
        out,
        "Opening browser to sign in\u{2026}\nIf the browser didn't open, visit: {link}\nPaste code here if prompted > "
    );
    let _ = out.flush();

    let mode = env::var("FAKE_CLAUDE_LOGIN").unwrap_or_else(|_| "hang".to_string());
    let (kind, value) = mode.split_once(':').unwrap_or((mode.as_str(), ""));
    match kind {
        "ok" | "quiet" => {
            thread::sleep(Duration::from_millis(300));
            write_login(profile, value);
            if kind == "ok" {
                let _ = writeln!(out, "Login successful.");
            }
            0
        }
        "code" => {
            for line in std::io::stdin().lock().lines() {
                let Ok(line) = line else { break };
                let parts: Vec<&str> = line.trim().split('#').collect();
                if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                    write_login(profile, value);
                    let _ = writeln!(out, "Login successful.");
                    return 0;
                }
                eprintln!("Invalid code. Please make sure the full code was copied.");
            }
            1
        }
        "nodisk" => {
            let _ = writeln!(out, "Login successful.");
            0
        }
        "fail" => {
            let _ = writeln!(out);
            eprintln!("Login failed: {value}");
            1
        }
        _ => loop {
            thread::sleep(Duration::from_secs(60));
        },
    }
}

/// O que o login de verdade deixa no perfil: a credencial (blob com a forma
/// da real, valores falsos) e a identidade no `.claude.json`.
fn write_login(profile: Option<&str>, email: &str) {
    let Some(dir) = profile.map(PathBuf::from) else {
        return;
    };
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(
        dir.join(".credentials.json"),
        r#"{"claudeAiOauth":{"accessToken":"falso-login"},"mcpOAuth":{}}"#,
    );
    let identity = serde_json::json!({
        "oauthAccount": {"emailAddress": email, "organizationName": "Acme"},
        "hasCompletedOnboarding": true,
    });
    let _ = fs::write(dir.join(".claude.json"), identity.to_string());
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
