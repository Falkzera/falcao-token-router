//! O que os comandos da CLI dividem: onde tudo mora, a configuração, a
//! credencial (com a guarda do perfil padrão) e o jeito de falhar.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use router_core::engine::anthropic_adapter::AnthropicAdapter;
use router_core::engine::credential_store::{CredentialStore, FileCredentialStore};
use router_core::engine::default_profile_guard::DefaultProfileGuard;
use router_core::engine::provider::ProviderAdapter;
use router_core::engine::router_paths::RouterPaths;
use router_core::engine::shell_integration::StatusShell;
use router_core::platform::atomic_write::read_retrying;
use router_core::platform::git_bash::{find_git_bash, GitBashEnv};
use router_core::RouterConfig;

/// A home do usuário (`%USERPROFILE%`). É dela que sai o perfil padrão
/// (`<home>\.claude`) — e é o que os testes trocam para nunca tocar o real.
pub fn home() -> String {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default()
}

/// A configuração, ou `None` se não existe (ou não se lê).
pub fn load_config(paths: &RouterPaths) -> Option<RouterConfig> {
    let data = read_retrying(&paths.config_file()).ok()?;
    serde_json::from_slice(&data).ok()
}

/// A credencial em arquivo, com a guarda: antes da 1ª escrita no `~\.claude`,
/// o login que estava lá é copiado para `<base>\backups\`.
pub fn credentials(paths: &RouterPaths, home: &str) -> Arc<dyn CredentialStore> {
    Arc::new(FileCredentialStore::with_guard(
        DefaultProfileGuard::for_home(home, &paths.base),
    ))
}

pub fn adapters() -> Vec<Arc<dyn ProviderAdapter>> {
    vec![Arc::new(AnthropicAdapter)]
}

/// O caminho deste `router.exe` — o que a integração e a status line citam.
pub fn self_path() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

/// O shell que o Claude Code vai usar para a status line (a mesma busca dele).
pub fn detected_shell() -> StatusShell {
    if find_git_bash(&GitBashEnv::from_process()).is_some() {
        StatusShell::Bash
    } else {
        StatusShell::PowerShell
    }
}

/// `router: <msg>` no stderr, código 1 — como o `fail` do macOS.
pub fn fail(message: &str) -> ! {
    eprintln!("router: {message}");
    std::process::exit(1);
}

/// Roda um comando curto com prazo e devolve (código, stdout). O stdout é lido
/// numa thread, para um processo que trava não segurar o `doctor`.
pub fn run_with_timeout(
    mut command: Command,
    stdin: Option<&[u8]>,
    timeout: Duration,
) -> Option<(Option<i32>, String)> {
    command
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command.spawn().ok()?;
    if let (Some(bytes), Some(mut pipe)) = (stdin, child.stdin.take()) {
        use std::io::Write;
        let _ = pipe.write_all(bytes);
        // Fecha o stdin: o sensor lê com prazo, mas não custa entregar o EOF.
    }
    let mut stdout = child.stdout.take()?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        let _ = tx.send(buf);
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(_) => return None,
        }
    }?;
    let out = rx.recv_timeout(Duration::from_secs(2)).unwrap_or_default();
    Some((status.code(), String::from_utf8_lossy(&out).into_owned()))
}
