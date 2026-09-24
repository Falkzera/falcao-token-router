//! O Git Bash que o Claude Code usa para rodar a status line — achado na MESMA
//! ordem que ele usa (lida no JS do binário 2.1.280): `CLAUDE_CODE_GIT_BASH_PATH`
//! → `%ProgramFiles%\Git\bin\bash.exe` → `%ProgramFiles(x86)%\Git\bin\bash.exe`
//! → o `git.exe` do `PATH` (o bash mora em `<Git>\bin`). Sem nenhum, ele usa o
//! PowerShell. Saber qual shell vai rodar o comando decide as aspas dele.

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// O que a busca consulta do ambiente — injetável para os testes.
#[derive(Clone, Debug, Default)]
pub struct GitBashEnv {
    pub override_path: Option<PathBuf>,
    pub program_files: Option<PathBuf>,
    pub program_files_x86: Option<PathBuf>,
    pub path: Option<OsString>,
}

impl GitBashEnv {
    pub fn from_process() -> Self {
        let var = |key: &str| env::var_os(key).filter(|v| !v.is_empty());
        GitBashEnv {
            override_path: var("CLAUDE_CODE_GIT_BASH_PATH").map(PathBuf::from),
            program_files: var("ProgramFiles").map(PathBuf::from),
            program_files_x86: var("ProgramFiles(x86)").map(PathBuf::from),
            path: var("PATH"),
        }
    }
}

/// O `bash.exe` que o Claude Code usaria, ou `None` (ele usaria o PowerShell).
pub fn find_git_bash(env: &GitBashEnv) -> Option<PathBuf> {
    // A variável só vale para um bash/sh que existe; com outro nome, o Claude
    // Code a ignora (e avisa) e segue a busca — visto no 2.1.280, 23/09/2026.
    let is_bash = |p: &Path| {
        p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
            ["bash.exe", "sh.exe", "bash", "sh"].contains(&n.to_lowercase().as_str())
        })
    };
    if let Some(p) = env
        .override_path
        .as_deref()
        .filter(|p| is_bash(p) && p.is_file())
    {
        return Some(p.to_path_buf());
    }
    let installed = [&env.program_files, &env.program_files_x86]
        .into_iter()
        .flatten()
        .map(|root| root.join("Git").join("bin").join("bash.exe"))
        .find(|p| p.is_file());
    if installed.is_some() {
        return installed;
    }
    let path = env.path.as_deref()?;
    env::split_paths(path)
        .filter(|dir| dir.join("git.exe").is_file())
        .find_map(|dir| bash_beside_git(&dir))
}

/// O `git.exe` do Git for Windows mora em `<Git>\cmd` (ou `<Git>\bin`); o bash,
/// em `<Git>\bin`.
fn bash_beside_git(git_dir: &Path) -> Option<PathBuf> {
    let root = git_dir.parent()?;
    [root.join("bin").join("bash.exe"), git_dir.join("bash.exe")]
        .into_iter()
        .find(|p| p.is_file())
}
