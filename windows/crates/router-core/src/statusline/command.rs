//! O modo "meu comando": depois do sensor, o router roda o comando que o
//! usuário deu, com o MESMO JSON, do jeito que o Claude Code rodaria a status
//! line (lido no JS do binário 2.1.280, 23/09/2026 — ela passa pelo executor
//! dos hooks):
//!
//! - Git Bash (`bash -c <comando>`), com a pasta do bash na frente do `PATH`;
//!   um comando cujo 1º termo é um `.sh` vira `bash <comando>`. Sem Git Bash,
//!   PowerShell (o 7 antes do 5.1) com `-NoProfile -NonInteractive
//!   -ExecutionPolicy Bypass -Command` — o `Bypass` sai com
//!   `CLAUDE_CODE_POWERSHELL_RESPECT_EXECUTION_POLICY`;
//! - sem janela; o JSON + `\n` no stdin, e o stdin FECHADO (EOF);
//! - vale a saída de quem sai com código 0 e imprime algo visível — o Claude
//!   Code mostra só isso (senão a linha fica vazia; aqui, vale a do app).
//!
//! Diferente dele, e de propósito: prazo de 5 s (o dele é o dos hooks, 10 min,
//! mais o cancelamento a cada atualização nova), e a ÁRVORE inteira morre no
//! prazo ou quando o router sai (`platform::job`) — o processo pendurado não
//! se acumula.

use std::io::{Read, Write};
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use windows_sys::Win32::Foundation::{HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{WaitForSingleObject, CREATE_NO_WINDOW};

use crate::platform::git_bash::{find_git_bash, GitBashEnv};
use crate::platform::job;
use crate::platform::powershell::{find_powershell, PowerShellEnv};

/// Quanto o comando do usuário tem para imprimir.
pub const DEADLINE: Duration = Duration::from_secs(5);

/// Marca o ambiente do comando: um `router statusline` rodando DENTRO dele
/// não roda o comando de novo (o usuário pode ter posto o próprio router).
pub const CHAINED_ENV: &str = "ROUTER_STATUSLINE_CHAINED";

/// Depois que o shell saiu, quanto esperar a saída fechar: um filho em segundo
/// plano que herdou o stdout a seguraria aberta.
const GRACE: Duration = Duration::from_millis(150);

/// Quanto do stderr volta (a tela mostra por que o comando falhou).
const STDERR_LIMIT: usize = 2000;

/// O shell que roda o comando da status line.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Shell {
    Bash(PathBuf),
    PowerShell(PathBuf),
}

impl Shell {
    /// O que o Claude Code usaria agora: Git Bash; sem ele, PowerShell.
    pub fn detect() -> Option<Shell> {
        find_git_bash(&GitBashEnv::from_process())
            .map(Shell::Bash)
            .or_else(|| find_powershell(&PowerShellEnv::from_process()).map(Shell::PowerShell))
    }

    /// O processo que roda `line` como o Claude Code o roda. `env` é o
    /// ambiente de onde vêm o `PATH` e a chave da política de execução.
    pub fn process(&self, line: &str, env: impl Fn(&str) -> Option<String>) -> Command {
        match self {
            Shell::Bash(bash) => {
                let mut process = Command::new(bash);
                process.arg("-c").arg(bash_line(line));
                // A pasta do bash na frente do `PATH`, como ele faz (`mnn`).
                if let Some(dir) = bash.parent().filter(|_| bash.is_absolute()) {
                    let current = env("PATH").unwrap_or_default();
                    process.env("PATH", path_led_by(dir, &current));
                }
                process
            }
            Shell::PowerShell(exe) => {
                let mut process = Command::new(exe);
                process.args(["-NoProfile", "-NonInteractive"]);
                let respect = env("CLAUDE_CODE_POWERSHELL_RESPECT_EXECUTION_POLICY")
                    .is_some_and(|v| !v.is_empty());
                if !respect {
                    process.args(["-ExecutionPolicy", "Bypass"]);
                }
                process.arg("-Command").arg(line);
                process
            }
        }
    }
}

/// O `PATH` com `dir` na frente — sem repetir, quando ele já está lá (o router
/// roda dentro da sessão, cujo `PATH` o Claude Code já montou assim).
fn path_led_by(dir: &Path, current: &str) -> String {
    let dir = dir.to_string_lossy();
    let first = current.split(';').next().unwrap_or_default();
    if first.eq_ignore_ascii_case(&dir) {
        current.to_string()
    } else if current.is_empty() {
        dir.into_owned()
    } else {
        format!("{dir};{current}")
    }
}

/// Um comando cujo 1º termo é um `.sh` roda como `bash <comando>` (o `gnn` do
/// Claude Code: aspas agrupam, `\` escapa o caractere seguinte).
pub fn bash_line(command: &str) -> String {
    let mut first = String::new();
    let mut chars = command.trim().chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' | '\'' => {
                let mut closed = false;
                for inside in chars.by_ref() {
                    if inside == c {
                        closed = true;
                        break;
                    }
                    first.push(inside);
                }
                if !closed {
                    break;
                }
            }
            '\\' if chars.peek().is_some() => first.extend(chars.next()),
            c if c.is_whitespace() => break,
            c => first.push(c),
        }
    }
    if first.ends_with(".sh") {
        format!("bash {command}")
    } else {
        command.to_string()
    }
}

/// O que aconteceu com o comando.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Outcome {
    /// Saiu com 0 e imprimiu algo visível: a linha, com os bytes como vieram.
    Printed(Vec<u8>),
    /// Rodou, mas o Claude Code não mostraria nada: código diferente de 0 ou
    /// saída em branco. Com o começo do stderr, que diz o porquê.
    Failed { code: Option<i32>, stderr: String },
    /// Nem subiu (o shell não existe mais…).
    NotStarted(String),
    /// Passou do prazo: a árvore inteira foi encerrada.
    TimedOut,
}

/// Roda o comando do usuário com a entrada (o JSON da status line), no shell
/// dado, com prazo.
pub fn run(shell: &Shell, command: &str, input: &[u8], deadline: Duration) -> Outcome {
    let mut process = shell.process(command, |key| std::env::var(key).ok());
    process.env(CHAINED_ENV, "1");
    run_process(process, input, deadline)
}

fn run_process(mut process: Command, input: &[u8], deadline: Duration) -> Outcome {
    process
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Sem janela, como o `windowsHide` dele: do app, cada teste piscaria uma.
    let (mut child, job) = match job::spawn_contained(&mut process, CREATE_NO_WINDOW) {
        Ok(started) => started,
        Err(e) => return Outcome::NotStarted(e.to_string()),
    };
    if let Some(mut stdin) = child.stdin.take() {
        let mut payload = input.to_vec();
        payload.push(b'\n');
        // Numa thread: um comando que não lê o stdin não segura ninguém. Ao
        // terminar, o pipe fecha — é o EOF.
        thread::spawn(move || {
            let _ = stdin.write_all(&payload);
        });
    }
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let cut = |child: &mut Child| match &job {
        Some(job) => job.terminate(),
        None => {
            let _ = child.kill();
        }
    };

    if !exits_within(&child, deadline) {
        cut(&mut child);
        let _ = child.wait();
        return Outcome::TimedOut;
    }
    let code = child.wait().ok().and_then(|status| status.code());
    let mut collect = |pipe: &Receiver<Vec<u8>>| match pipe.recv_timeout(GRACE) {
        Ok(bytes) => bytes,
        Err(_) => {
            cut(&mut child);
            pipe.recv_timeout(Duration::from_secs(1))
                .unwrap_or_default()
        }
    };
    let out = collect(&stdout);
    let err = collect(&stderr);
    drop(job);

    if code == Some(0) && !String::from_utf8_lossy(&out).trim().is_empty() {
        Outcome::Printed(out)
    } else {
        let text = String::from_utf8_lossy(&err);
        Outcome::Failed {
            code,
            stderr: text.trim().chars().take(STDERR_LIMIT).collect(),
        }
    }
}

/// Lê um pipe até o fim, numa thread.
fn drain(pipe: Option<impl Read + Send + 'static>) -> Receiver<Vec<u8>> {
    let (tx, rx) = mpsc::channel();
    match pipe {
        Some(mut pipe) => {
            thread::spawn(move || {
                let mut bytes = Vec::new();
                let _ = pipe.read_to_end(&mut bytes);
                let _ = tx.send(bytes);
            });
        }
        None => {
            let _ = tx.send(Vec::new());
        }
    }
    rx
}

/// O processo saiu dentro do prazo? (Espera no handle dele: acorda na hora.)
fn exits_within(child: &Child, deadline: Duration) -> bool {
    // `u32::MAX` é "para sempre" para o Windows.
    let ms = u32::try_from(deadline.as_millis()).unwrap_or(u32::MAX - 1);
    // SAFETY: o handle do processo vale enquanto `child` vive.
    unsafe { WaitForSingleObject(child.as_raw_handle() as HANDLE, ms) == WAIT_OBJECT_0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::job::test_support::{running, within};
    use crate::platform::powershell::{find_powershell, PowerShellEnv};
    use std::ffi::OsStr;
    use std::os::windows::process::CommandExt;
    use std::time::Instant;

    // O 1º termo do comando (`gnn` no JS do Claude Code).

    #[test]
    fn a_shell_script_first_runs_through_bash() {
        assert_eq!(bash_line("~/linha.sh --curta"), "bash ~/linha.sh --curta");
        assert_eq!(
            bash_line("'C:/Meus Scripts/linha.sh' x"),
            "bash 'C:/Meus Scripts/linha.sh' x"
        );
        // Barra invertida escapa o caractere seguinte, como no parser dele.
        assert_eq!(bash_line(r"C:\x\linha.sh"), r"bash C:\x\linha.sh");
        // Aspas sem fecho: o resto é o termo.
        assert_eq!(bash_line("'linha.sh"), "bash 'linha.sh");
        // O comando vai como veio (espaços inclusos), só com o `bash ` na frente.
        assert_eq!(bash_line("  ./linha.sh  "), "bash   ./linha.sh  ");
    }

    #[test]
    fn anything_else_goes_as_it_came() {
        assert_eq!(
            bash_line("node C:/Users/exemplo/linha.js"),
            "node C:/Users/exemplo/linha.js"
        );
        // A comparação dele é com `.sh` minúsculo.
        assert_eq!(bash_line("linha.SH"), "linha.SH");
        // Só o 1º termo conta.
        assert_eq!(bash_line("node linha.js x.sh"), "node linha.js x.sh");
        assert_eq!(bash_line(""), "");
    }

    fn args(command: &Command) -> Vec<String> {
        command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    fn env_of<'a>(command: &'a Command, key: &str) -> Option<&'a OsStr> {
        command
            .get_envs()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .and_then(|(_, v)| v)
    }

    #[test]
    fn bash_runs_the_line_with_its_folder_first_on_the_path() {
        let bash = Shell::Bash(PathBuf::from(r"C:\Git\bin\bash.exe"));
        let env = |key: &str| (key == "PATH").then(|| r"C:\x;C:\y".to_string());
        let process = bash.process("~/linha.sh", env);
        assert_eq!(process.get_program(), r"C:\Git\bin\bash.exe");
        assert_eq!(args(&process), ["-c", "bash ~/linha.sh"]);
        assert_eq!(env_of(&process, "PATH").unwrap(), r"C:\Git\bin;C:\x;C:\y");

        // Já na frente (o router rodando dentro da sessão): não repete.
        let env = |key: &str| (key == "PATH").then(|| r"c:\git\bin;C:\x".to_string());
        let process = bash.process("node linha.js", env);
        assert_eq!(env_of(&process, "PATH").unwrap(), r"c:\git\bin;C:\x");
    }

    #[test]
    fn powershell_runs_the_line_like_claude_code_does() {
        let pwsh = Shell::PowerShell(PathBuf::from(r"C:\PS\pwsh.exe"));
        let process = pwsh.process("& 'C:/x/linha.ps1'", |_: &str| None);
        assert_eq!(process.get_program(), r"C:\PS\pwsh.exe");
        assert_eq!(
            args(&process),
            [
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "& 'C:/x/linha.ps1'"
            ]
        );
        let respect = |key: &str| {
            (key == "CLAUDE_CODE_POWERSHELL_RESPECT_EXECUTION_POLICY").then(|| "1".to_string())
        };
        assert_eq!(
            args(&pwsh.process("x", respect)),
            ["-NoProfile", "-NonInteractive", "-Command", "x"]
        );
    }

    fn powershell() -> Shell {
        Shell::PowerShell(find_powershell(&PowerShellEnv::from_process()).expect("PowerShell"))
    }

    /// Ponta a ponta, no PowerShell de verdade: o comando lê o JSON até o fim
    /// (só termina com o EOF) e sabe que roda dentro do router.
    #[test]
    fn the_command_reads_the_json_to_the_end_and_prints_the_line() {
        let command = "$j = [Console]::In.ReadToEnd() | ConvertFrom-Json; \
                       Write-Output ('modelo: ' + $j.model.display_name + ' encadeado: ' + $env:ROUTER_STATUSLINE_CHAINED)";
        let outcome = run(
            &powershell(),
            command,
            br#"{"model":{"display_name":"Opus 5.5"}}"#,
            Duration::from_secs(20),
        );
        let Outcome::Printed(line) = outcome else {
            panic!("{outcome:?}");
        };
        assert_eq!(
            String::from_utf8_lossy(&line).trim(),
            "modelo: Opus 5.5 encadeado: 1"
        );
    }

    #[test]
    fn a_shell_that_does_not_exist_does_not_start() {
        let outcome = run(
            &Shell::Bash(PathBuf::from(r"C:\nao\existe\bash.exe")),
            "echo x",
            b"{}",
            Duration::from_secs(2),
        );
        assert!(matches!(outcome, Outcome::NotStarted(_)), "{outcome:?}");
    }

    // O resto roda pelo `cmd`, que sobe em milissegundos.

    fn cmd(line: &str) -> Command {
        let mut command = Command::new("cmd.exe");
        command.raw_arg(format!("/d /c {line}"));
        command
    }

    #[test]
    fn a_line_comes_back_as_it_was_printed() {
        assert_eq!(
            run_process(cmd("echo linha"), b"{}", Duration::from_secs(5)),
            Outcome::Printed(b"linha\r\n".to_vec())
        );
    }

    #[test]
    fn a_failure_brings_its_code_and_what_it_said() {
        let outcome = run_process(
            cmd("echo quebrou 1>&2 & exit /b 3"),
            b"{}",
            Duration::from_secs(5),
        );
        let Outcome::Failed { code, stderr } = outcome else {
            panic!("{outcome:?}");
        };
        assert_eq!(code, Some(3));
        assert!(stderr.contains("quebrou"), "{stderr}");
    }

    /// O Claude Code não mostra a saída de quem sai com código diferente de 0.
    #[test]
    fn output_with_a_nonzero_exit_is_not_a_line() {
        let outcome = run_process(cmd("echo linha & exit /b 1"), b"{}", Duration::from_secs(5));
        assert!(
            matches!(outcome, Outcome::Failed { code: Some(1), .. }),
            "{outcome:?}"
        );
    }

    #[test]
    fn blank_output_is_not_a_line() {
        for line in ["exit /b 0", "echo.", "echo.   "] {
            let outcome = run_process(cmd(line), b"{}", Duration::from_secs(5));
            assert!(
                matches!(outcome, Outcome::Failed { code: Some(0), .. }),
                "{line}: {outcome:?}"
            );
        }
    }

    /// No prazo, a ÁRVORE morre — não só o shell. O neto aqui é um `waitfor`
    /// (só este teste o usa), que esperaria 30 s.
    #[test]
    fn past_the_deadline_the_whole_tree_is_cut() {
        let start = Instant::now();
        let outcome = run_process(
            // O nome do sinal só aceita letras e dígitos.
            cmd("waitfor /t 30 FalcaoRouterTestePrazo"),
            b"{}",
            Duration::from_millis(700),
        );
        assert_eq!(outcome, Outcome::TimedOut);
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "{:?}",
            start.elapsed()
        );
        assert!(
            within(Duration::from_secs(3), || (!running("waitfor.exe"))
                .then_some(()))
            .is_some(),
            "o neto sobrou"
        );
    }

    /// Um filho em segundo plano que herdou o stdout seguraria a saída até
    /// morrer; a linha já impressa vale, e ele vai junto.
    #[test]
    fn a_background_child_holding_the_output_does_not_hold_the_line() {
        let start = Instant::now();
        let outcome = run_process(
            cmd(r#"start /b cmd /d /c "ping -n 30 127.0.0.1 >nul" & echo linha"#),
            b"{}",
            Duration::from_secs(5),
        );
        let Outcome::Printed(line) = outcome else {
            panic!("{outcome:?}");
        };
        assert_eq!(String::from_utf8_lossy(&line).trim(), "linha");
        assert!(
            start.elapsed() < Duration::from_secs(4),
            "{:?}",
            start.elapsed()
        );
    }
}
