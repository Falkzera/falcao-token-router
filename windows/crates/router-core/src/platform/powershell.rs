//! O PowerShell que o Claude Code usa para a status line quando não há Git
//! Bash — achado na MESMA ordem que ele usa (lida no JS do binário 2.1.280,
//! 23/09/2026): `pwsh` no `PATH` → `%ProgramFiles%\PowerShell\7\pwsh.exe` →
//! `%LOCALAPPDATA%\Microsoft\WindowsApps\pwsh.exe` (o da Store) →
//! `%USERPROFILE%\.dotnet\tools\pwsh.exe` → `powershell` no `PATH` →
//! `%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe`. O 7 vem
//! antes do 5.1.

use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

/// O que a busca consulta do ambiente — injetável para os testes.
#[derive(Clone, Debug, Default)]
pub struct PowerShellEnv {
    pub path: Option<OsString>,
    pub program_files: Option<PathBuf>,
    pub local_app_data: Option<PathBuf>,
    pub user_profile: Option<PathBuf>,
    pub system_root: Option<PathBuf>,
}

impl PowerShellEnv {
    pub fn from_process() -> Self {
        let var = |key: &str| env::var_os(key).filter(|v| !v.is_empty());
        PowerShellEnv {
            path: var("PATH"),
            program_files: var("ProgramFiles").map(PathBuf::from),
            local_app_data: var("LOCALAPPDATA").map(PathBuf::from),
            user_profile: var("USERPROFILE").map(PathBuf::from),
            system_root: var("SystemRoot").map(PathBuf::from),
        }
    }
}

/// O `pwsh.exe`/`powershell.exe` que o Claude Code usaria, ou `None`.
pub fn find_powershell(env: &PowerShellEnv) -> Option<PathBuf> {
    let on_path = |exe: &str| {
        let path = env.path.as_deref()?;
        env::split_paths(path)
            .map(|dir| dir.join(exe))
            .find(|candidate| exists(candidate))
    };
    let under = |root: &Option<PathBuf>, parts: &[&str]| {
        let mut candidate = root.clone()?;
        candidate.extend(parts);
        exists(&candidate).then_some(candidate)
    };
    on_path("pwsh.exe")
        .or_else(|| under(&env.program_files, &["PowerShell", "7", "pwsh.exe"]))
        .or_else(|| {
            under(
                &env.local_app_data,
                &["Microsoft", "WindowsApps", "pwsh.exe"],
            )
        })
        .or_else(|| under(&env.user_profile, &[".dotnet", "tools", "pwsh.exe"]))
        .or_else(|| on_path("powershell.exe"))
        .or_else(|| {
            let root = Some(
                env.system_root
                    .clone()
                    .unwrap_or_else(|| PathBuf::from(r"C:\Windows")),
            );
            under(
                &root,
                &["System32", "WindowsPowerShell", "v1.0", "powershell.exe"],
            )
        })
}

/// Existe como arquivo — inclusive o atalho de execução da Store (em
/// `WindowsApps`), um ponto de reparação que o `metadata` comum não abre.
fn exists(path: &std::path::Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| !m.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn touch(path: &Path) -> PathBuf {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"").unwrap();
        path.to_path_buf()
    }

    /// Uma máquina com TODOS os lugares preenchidos; cada teste apaga o que
    /// vem antes do lugar que ele quer ver ganhar.
    struct Machine {
        _tmp: tempfile::TempDir,
        env: PowerShellEnv,
        on_path_7: PathBuf,
        program_files_7: PathBuf,
        store_7: PathBuf,
        dotnet_7: PathBuf,
        on_path_5: PathBuf,
        system_5: PathBuf,
    }

    fn machine() -> Machine {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let bin7 = root.join("bin7");
        let bin5 = root.join("bin5");
        // A pasta temporária entra por último: os campos acima usam o `root`.
        Machine {
            on_path_7: touch(&bin7.join("pwsh.exe")),
            program_files_7: touch(
                &root
                    .join("pf")
                    .join("PowerShell")
                    .join("7")
                    .join("pwsh.exe"),
            ),
            store_7: touch(
                &root
                    .join("local")
                    .join("Microsoft")
                    .join("WindowsApps")
                    .join("pwsh.exe"),
            ),
            dotnet_7: touch(
                &root
                    .join("home")
                    .join(".dotnet")
                    .join("tools")
                    .join("pwsh.exe"),
            ),
            on_path_5: touch(&bin5.join("powershell.exe")),
            system_5: touch(
                &root
                    .join("win")
                    .join("System32")
                    .join("WindowsPowerShell")
                    .join("v1.0")
                    .join("powershell.exe"),
            ),
            env: PowerShellEnv {
                path: Some(env::join_paths([root.join("vazio"), bin5, bin7]).unwrap()),
                program_files: Some(root.join("pf")),
                local_app_data: Some(root.join("local")),
                user_profile: Some(root.join("home")),
                system_root: Some(root.join("win")),
            },
            _tmp: tmp,
        }
    }

    #[test]
    fn the_powershell_7_on_the_path_comes_first() {
        let m = machine();
        // Mesmo com o 5.1 antes dele no PATH: o 7 ganha.
        assert_eq!(find_powershell(&m.env), Some(m.on_path_7.clone()));
    }

    #[test]
    fn then_the_places_where_powershell_7_installs() {
        let m = machine();
        std::fs::remove_file(&m.on_path_7).unwrap();
        assert_eq!(find_powershell(&m.env), Some(m.program_files_7.clone()));
        std::fs::remove_file(&m.program_files_7).unwrap();
        assert_eq!(find_powershell(&m.env), Some(m.store_7.clone()));
        std::fs::remove_file(&m.store_7).unwrap();
        assert_eq!(find_powershell(&m.env), Some(m.dotnet_7.clone()));
    }

    #[test]
    fn windows_powershell_is_the_fallback() {
        let m = machine();
        for p in [&m.on_path_7, &m.program_files_7, &m.store_7, &m.dotnet_7] {
            std::fs::remove_file(p).unwrap();
        }
        assert_eq!(find_powershell(&m.env), Some(m.on_path_5.clone()));
        std::fs::remove_file(&m.on_path_5).unwrap();
        assert_eq!(find_powershell(&m.env), Some(m.system_5.clone()));
        std::fs::remove_file(&m.system_5).unwrap();
        assert_eq!(find_powershell(&m.env), None);
    }

    /// Nesta máquina (e na CI) há sempre ao menos o 5.1.
    #[test]
    fn the_real_machine_has_one() {
        assert!(find_powershell(&PowerShellEnv::from_process()).is_some());
    }
}
