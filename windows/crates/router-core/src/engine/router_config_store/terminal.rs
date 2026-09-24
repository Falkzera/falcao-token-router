//! Integração com o terminal: os scripts de shell e a status line.
//!
//! Parte do `impl RouterConfigStore` — ver o `mod.rs` ao lado.

use super::*;

impl RouterConfigStore {
    // MARK: - Integração com o terminal

    pub fn set_router_path(&mut self, path: Option<PathBuf>) {
        self.router_path = path;
    }

    pub fn router_path(&self) -> Option<&Path> {
        self.router_path.as_deref()
    }

    /// O `shell.ps1` que os perfis do PowerShell carregam.
    pub fn powershell_script_path(&self) -> PathBuf {
        self.paths.base.join("shell.ps1")
    }

    /// O `shell.sh` que o `~\.bashrc` do Git Bash carrega.
    pub fn bash_script_path(&self) -> PathBuf {
        self.paths.base.join("shell.sh")
    }

    /// A status line que o perfil de um grupo deve ter, para o shell que o
    /// Claude Code vai usar.
    pub fn expected_status_line(&self, group: &AccountGroup, shell: StatusShell) -> Option<String> {
        let router = self.router_path.as_deref()?;
        Some(ShellIntegration::status_line_for_profile(
            router,
            &group.config_dir,
            shell,
        ))
    }

    /// Escreve os scripts, planta a status line e o compartilhamento em cada
    /// perfil de grupo, e acrescenta a linha nos perfis de shell. Idempotente.
    /// A parte do Git Bash só entra quando há Git Bash (`shell == Bash`).
    pub fn install_shell_integration(
        &mut self,
        targets: &ShellTargets,
        shell: StatusShell,
    ) -> Result<(), StoreError> {
        let Some(router) = self.router_path.clone() else {
            self.last_error = Some(StoreError::RouterPathUnknown);
            return Err(StoreError::RouterPathUnknown);
        };
        let (ps1, sh) = (self.powershell_script_path(), self.bash_script_path());
        let mut failures: Vec<String> = Vec::new();

        if let Err(e) = ShellIntegration::write_scripts(&router, &ps1, &sh) {
            failures.push(e.to_string());
        }
        let home = ConfigDir::standard(&self.home);
        for group in &self.config.groups {
            if let Some(command) = self.expected_status_line(group, shell) {
                if let Err(e) = ShellIntegration::install_status_line(&command, &group.config_dir) {
                    failures.push(e.to_string());
                }
            }
            ProfileSharing::link(
                &group.config_dir,
                &home,
                self.config.share_history,
                &self.paths.base,
            );
        }
        let ps_line = ShellIntegration::powershell_source_line(&ps1);
        for profile in &targets.powershell_profiles {
            if let Err(e) = ShellIntegration::ensure_in_profile(profile, &ps_line, "\r\n") {
                failures.push(format!("{}: {e}", profile.display()));
            }
        }
        if shell == StatusShell::Bash {
            let sh_line = ShellIntegration::bash_source_line(&sh);
            if let Err(e) = ShellIntegration::ensure_in_profile(&targets.bashrc, &sh_line, "\n") {
                failures.push(format!("{}: {e}", targets.bashrc.display()));
            }
            if let Err(e) = ShellIntegration::ensure_bash_profile(targets) {
                failures.push(e.to_string());
            }
        }

        if failures.is_empty() {
            self.last_error = None;
            Ok(())
        } else {
            let error = StoreError::IntegrationFailed(failures.join("; "));
            self.last_error = Some(error.clone());
            Err(error)
        }
    }

    /// A integração instalada aponta para um binário que não é mais este (app
    /// movido ou reinstalado noutro lugar), ou algum perfil de grupo está sem a
    /// status line certa. O modo de falha é silencioso — `claude trabalho` cai
    /// no `claude` puro, na conta errada — por isso a cura é automática.
    /// Não instalada não é obsoleta: é ausente.
    pub fn integration_is_stale(&self, shell: StatusShell) -> bool {
        let Some(router) = self.router_path.as_deref() else {
            return false;
        };
        let ps1 = self.powershell_script_path();
        let Ok(script) = read_retrying(&ps1) else {
            return false;
        };
        let router_text = router.to_string_lossy().replace('\'', "''");
        if !String::from_utf8_lossy(&script).contains(router_text.as_str()) {
            return true;
        }
        self.config.groups.iter().any(|group| {
            self.expected_status_line(group, shell)
                .is_some_and(|cmd| ShellIntegration::status_line_is_stale(&cmd, &group.config_dir))
        })
    }

    /// Reinstala quando ficou obsoleta. Devolve `true` se mexeu em algo.
    pub fn heal_shell_integration(&mut self, targets: &ShellTargets, shell: StatusShell) -> bool {
        self.integration_is_stale(shell) && self.install_shell_integration(targets, shell).is_ok()
    }
}
