//! Rodar um comando curto com prazo — para consultas que não podem segurar quem
//! pergunta (o `doctor` na linha de comando, a tela de Grupos no app).
//!
//! Saiu da CLI na fase 5: o app faz as mesmas consultas (política de execução
//! de cada PowerShell) e um processo que trava não pode congelar nenhum dos dois.

use std::io::Read;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Não abre console: do app (GUI) cada consulta piscaria uma janela preta.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Roda um comando com prazo e devolve (código, stdout). O stdout é lido numa
/// thread, para um processo que trava não segurar quem chamou; estourado o
/// prazo, o processo é morto e a resposta é `None`.
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
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW);
    let mut child = command.spawn().ok()?;
    if let (Some(bytes), Some(mut pipe)) = (stdin, child.stdin.take()) {
        use std::io::Write;
        let _ = pipe.write_all(bytes);
        // O `pipe` sai de escopo aqui e fecha o stdin: o sensor lê com prazo,
        // mas não custa entregar o EOF.
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
