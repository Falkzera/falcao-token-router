//! Onde está o binário oficial `claude` — um resolvedor SÓ (≙ `ClaudeBinary`;
//! o macOS tem três buscas diferentes, uma no login, uma na sonda e o `execvp`
//! do `launch`, que podem discordar).
//!
//! Ordem: `ROUTER_CLAUDE_BIN` (override explícito; os testes de integração o
//! usam para o `fake-claude`) → o instalador nativo
//! (`%USERPROFILE%\.local\bin\claude.exe`, onde o 2.1.280 mora) → cada pasta do
//! `PATH` (`claude.exe`, depois `claude.cmd`) → o npm global (`%APPDATA%\npm`),
//! para o app aberto com um `PATH` mínimo.
//!
//! O shim `claude.cmd` do npm é LIDO, não executado: vira `node` + `cli.js` (ou
//! o `.exe` do pacote). Rodar um `.cmd` passa pelo `cmd.exe`, que reinterpreta
//! `&`, `|` e `%` nos argumentos do usuário.
//!
//! Nunca escolhidos: a cópia do Claude Desktop (`…\AnthropicClaude\…`) e os
//! aliases do WindowsApps — não usam a credencial do perfil que o router
//! gerencia; perguntar o uso a eles responderia por outra conta, ou nenhuma.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::platform::paths::normalized;

/// Como rodar o `claude`: o programa e o que vai antes dos argumentos do usuário
/// (o `cli.js`, quando o programa é o `node`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ClaudeCommand {
    pub program: PathBuf,
    pub prefix_args: Vec<PathBuf>,
}

impl ClaudeCommand {
    /// Um `Command` pronto para receber os argumentos do usuário.
    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.prefix_args);
        command
    }

    /// Para o `doctor` mostrar o que vai rodar.
    pub fn describe(&self) -> String {
        std::iter::once(self.program.display().to_string())
            .chain(self.prefix_args.iter().map(|p| p.display().to_string()))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// O que a busca consulta do ambiente — injetável para os testes.
#[derive(Clone, Debug, Default)]
pub struct LocateEnv {
    pub override_bin: Option<PathBuf>,
    pub user_profile: Option<PathBuf>,
    pub path: Option<OsString>,
    pub appdata: Option<PathBuf>,
}

impl LocateEnv {
    pub fn from_process() -> Self {
        let var = |key: &str| env::var_os(key).filter(|v| !v.is_empty());
        LocateEnv {
            override_bin: var("ROUTER_CLAUDE_BIN").map(PathBuf::from),
            user_profile: var("USERPROFILE").map(PathBuf::from),
            path: var("PATH"),
            appdata: var("APPDATA").map(PathBuf::from),
        }
    }
}

pub struct ClaudeBinary;

impl ClaudeBinary {
    /// O `claude` desta máquina, ou `None` se não está instalado em lugar nenhum
    /// conhecido.
    pub fn locate() -> Option<ClaudeCommand> {
        Self::locate_in(&LocateEnv::from_process())
    }

    pub fn locate_in(env: &LocateEnv) -> Option<ClaudeCommand> {
        if let Some(bin) = env.override_bin.as_deref().filter(|b| b.is_file()) {
            return Self::command_for(bin, env);
        }
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(home) = &env.user_profile {
            candidates.push(home.join(".local").join("bin").join("claude.exe"));
        }
        for dir in Self::path_dirs(env) {
            candidates.push(dir.join("claude.exe"));
            candidates.push(dir.join("claude.cmd"));
        }
        if let Some(appdata) = &env.appdata {
            candidates.push(appdata.join("npm").join("claude.cmd"));
        }
        candidates
            .iter()
            .filter(|c| c.is_file() && !Self::is_foreign(c))
            .find_map(|c| Self::command_for(c, env))
    }

    fn path_dirs(env: &LocateEnv) -> Vec<PathBuf> {
        env.path
            .as_deref()
            .map(|p| {
                env::split_paths(p)
                    .filter(|d| !d.as_os_str().is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Cópia que não serve: a do Claude Desktop ou um alias do WindowsApps.
    fn is_foreign(path: &Path) -> bool {
        let p = normalized(path);
        p.contains("\\windowsapps\\") || p.contains("\\anthropicclaude\\")
    }

    fn command_for(path: &Path, env: &LocateEnv) -> Option<ClaudeCommand> {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase());
        match ext.as_deref() {
            Some("cmd") | Some("bat") => Self::from_shim(path, env),
            _ => Some(ClaudeCommand {
                program: path.to_path_buf(),
                prefix_args: Vec::new(),
            }),
        }
    }

    /// O shim do npm resolvido: `node` + script, ou o `.exe` que ele chama.
    fn from_shim(shim: &Path, env: &LocateEnv) -> Option<ClaudeCommand> {
        let text = fs::read_to_string(shim).ok()?;
        let dir = shim.parent()?;
        let target = dir.join(Self::shim_target(&text)?);
        if !target.is_file() || Self::is_foreign(&target) {
            return None;
        }
        let ext = target
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if matches!(ext.as_str(), "js" | "mjs" | "cjs") {
            // O shim prefere o `node.exe` ao lado dele; senão, o do PATH.
            let node = std::iter::once(dir.join("node.exe"))
                .chain(Self::path_dirs(env).into_iter().map(|d| d.join("node.exe")))
                .find(|n| n.is_file() && !Self::is_foreign(n))?;
            Some(ClaudeCommand {
                program: node,
                prefix_args: vec![target],
            })
        } else {
            Some(ClaudeCommand {
                program: target,
                prefix_args: Vec::new(),
            })
        }
    }

    /// O alvo de um shim do npm (cmd-shim): o último `"%dp0%\…"` que não é o
    /// `node.exe` que o próprio shim testa. Caminho relativo à pasta do shim.
    pub fn shim_target(text: &str) -> Option<String> {
        const MARK: &str = "\"%dp0%\\";
        let mut found = None;
        let mut rest = text;
        while let Some(start) = rest.find(MARK) {
            let after = &rest[start + MARK.len()..];
            let end = after.find('"')?;
            let candidate = after[..end].trim_start_matches('\\');
            if !candidate.eq_ignore_ascii_case("node.exe") {
                found = Some(candidate.to_string());
            }
            rest = &after[end + 1..];
        }
        found
    }
}
