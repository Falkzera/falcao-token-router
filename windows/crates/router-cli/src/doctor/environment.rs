//! O que o ambiente desvia, e o `claude` instalado.
//!
//! Checagens do `router doctor` — a ordem está no `mod.rs` ao lado.

use super::*;

/// Variáveis que desviam a sessão da conta do grupo. As sessões de grupo as
/// removem; o `claude` puro (grupo padrão) não. Só os NOMES são mostrados.
pub(super) fn check_environment(report: &mut Report) {
    let mut names: Vec<String> = std::env::vars_os()
        .map(|(k, _)| k)
        .filter(|k| ProviderEnv::redirects(k))
        .map(|k| k.to_string_lossy().into_owned())
        .collect();
    names.sort();
    if names.is_empty() {
        report.check(true, "nenhuma variável de proxy ou credencial no ambiente");
    } else {
        report.check(
            false,
            format!(
                "variáveis que desviam a sessão estão no ambiente: {} — as sessões de grupo as removem, mas o `claude` puro não",
                names.join(", ")
            ),
        );
    }
}

pub(super) fn check_claude(report: &mut Report) {
    let Some(claude) = ClaudeBinary::locate() else {
        report.check(
            false,
            "binário `claude` não encontrado — o `launch` e a sonda precisam dele",
        );
        return;
    };
    let mut command = claude.command();
    command.arg("--version");
    let version = shared::run_with_timeout(command, None, Duration::from_secs(30))
        .and_then(|(code, out)| (code == Some(0)).then_some(out))
        .and_then(|out| out.lines().next().map(|l| l.trim().to_string()))
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| "versão desconhecida".to_string());
    report.check(true, format!("claude: {} ({version})", claude.describe()));
}
