//! O que os comandos da CLI dividem: onde tudo mora, a configuração, a
//! credencial (com a guarda do perfil padrão) e o jeito de falhar.

use std::path::PathBuf;
use std::sync::Arc;

pub use router_core::platform::process::run_with_timeout;

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
