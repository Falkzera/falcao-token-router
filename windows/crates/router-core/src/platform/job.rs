//! A árvore inteira de um processo, para encerrar de uma vez (Job Object).
//!
//! O `Child::kill` (`TerminateProcess`) mata só o processo direto. Um comando
//! de status line roda como `bash -c` → `node` (ou `pwsh` → …), e o neto que
//! trava sobra — no Windows não há grupo de processos; é o Job Object que junta
//! a árvore. O processo nasce SUSPENSO, entra no job e só então é solto:
//! nenhum neto nasce fora dele. O job mata o que sobrar dentro dele quando é
//! fechado (`KILL_ON_JOB_CLOSE`) — inclusive quando quem o criou morre (o
//! Claude Code cancela a status line em curso a cada atualização nova).

use std::ffi::c_void;
use std::io;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::process::{Child, Command};

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Threading::{
    OpenThread, ResumeThread, CREATE_SUSPENDED, THREAD_SUSPEND_RESUME,
};

/// Um Job Object que mata o que sobrar dentro dele ao ser fechado.
pub struct Job {
    handle: HANDLE,
}

impl Job {
    fn new() -> io::Result<Job> {
        // SAFETY: job anônimo, sem atributos; nulo = falha.
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let job = Job { handle };
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: a estrutura é nossa e o tamanho é o dela.
        let ok = unsafe {
            SetInformationJobObject(
                job.handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } != 0;
        if ok {
            Ok(job)
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn assign(&self, child: &Child) -> bool {
        // SAFETY: os dois handles são válidos enquanto `self` e `child` vivem.
        unsafe { AssignProcessToJobObject(self.handle, child.as_raw_handle() as HANDLE) != 0 }
    }

    /// Encerra todos os processos do job, de uma vez.
    pub fn terminate(&self) {
        // SAFETY: handle nosso; código de saída 1, como o de um processo morto.
        unsafe { TerminateJobObject(self.handle, 1) };
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        // SAFETY: handle nosso, fechado uma vez só. Com `KILL_ON_JOB_CLOSE`, o
        // que sobrou dentro do job morre aqui.
        unsafe { CloseHandle(self.handle) };
    }
}

/// Sobe `command` (com as `flags` de criação dadas) dentro de um job novo. Se o
/// sistema não der o job (um job acima que não aceita aninhar), o processo
/// sobe mesmo assim, sem ele: rodar sem a rede de proteção é melhor que não
/// rodar — e o chamador ainda tem o `Child::kill`.
pub fn spawn_contained(command: &mut Command, flags: u32) -> io::Result<(Child, Option<Job>)> {
    let job = Job::new().ok();
    command.creation_flags(flags | CREATE_SUSPENDED);
    let mut child = command.spawn()?;
    let job = job.filter(|job| job.assign(&child));
    if let Err(e) = resume(child.id()) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(e);
    }
    Ok((child, job))
}

/// Solta a thread de um processo criado suspenso. O `std` não entrega o handle
/// dela; a lista de threads do sistema (Toolhelp) entrega o id.
fn resume(pid: u32) -> io::Result<()> {
    // SAFETY: foto da lista de threads; o handle é fechado no fim.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let mut entry = THREADENTRY32 {
        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };
    let mut resumed = 0;
    // SAFETY: `entry` é nosso, com o `dwSize` preenchido.
    let mut more = unsafe { Thread32First(snapshot, &mut entry) } != 0;
    while more {
        if entry.th32OwnerProcessID == pid {
            // SAFETY: só o direito de suspender/soltar; nulo = falha.
            let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
            if !thread.is_null() {
                // SAFETY: handle nosso, fechado logo depois.
                if unsafe { ResumeThread(thread) } != u32::MAX {
                    resumed += 1;
                }
                unsafe { CloseHandle(thread) };
            }
        }
        // SAFETY: idem.
        more = unsafe { Thread32Next(snapshot, &mut entry) } != 0;
    }
    // SAFETY: o handle é nosso.
    unsafe { CloseHandle(snapshot) };
    if resumed > 0 {
        Ok(())
    } else {
        Err(io::Error::other(
            "a thread do processo novo não apareceu para ser solta",
        ))
    }
}

/// Para os testes de quem sobe processos: ver a árvore pela lista do sistema.
#[cfg(test)]
pub(crate) mod test_support {
    use crate::platform::process_times::{probe, ProcessState};
    use std::thread;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    /// `(pid, pid do pai, nome do exe)` de cada processo vivo.
    pub fn processes() -> Vec<(u32, u32, String)> {
        // SAFETY: foto da lista de processos; o handle é fechado no fim.
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return Vec::new();
        }
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut all = Vec::new();
        // SAFETY: `entry` é nosso, com o `dwSize` preenchido.
        let mut more = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;
        while more {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            all.push((
                entry.th32ProcessID,
                entry.th32ParentProcessID,
                String::from_utf16_lossy(&entry.szExeFile[..len]),
            ));
            // SAFETY: idem.
            more = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
        }
        // SAFETY: o handle é nosso.
        unsafe { CloseHandle(snapshot) };
        all
    }

    /// O filho `exe` de `parent`.
    pub fn child_of(parent: u32, exe: &str) -> Option<u32> {
        processes()
            .into_iter()
            .find(|(_, p, name)| *p == parent && name.eq_ignore_ascii_case(exe))
            .map(|(pid, _, _)| pid)
    }

    /// Algum processo com esse exe está vivo?
    pub fn running(exe: &str) -> bool {
        processes()
            .iter()
            .any(|(_, _, name)| name.eq_ignore_ascii_case(exe))
    }

    pub fn within<T>(limit: Duration, mut check: impl FnMut() -> Option<T>) -> Option<T> {
        let start = Instant::now();
        loop {
            if let Some(value) = check() {
                return Some(value);
            }
            if start.elapsed() > limit {
                return None;
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn gone(pid: u32) -> bool {
        within(Duration::from_secs(3), || {
            (probe(pid) == ProcessState::Missing).then_some(())
        })
        .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{child_of, gone, within};
    use super::*;
    use std::time::Duration;
    use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

    /// Um `cmd` que deixa um `ping` (o neto) rodando por um minuto.
    fn tree() -> (Child, Option<Job>, u32) {
        let mut command = Command::new("cmd.exe");
        command.args(["/d", "/c", "ping -n 60 127.0.0.1 >nul"]);
        let (child, job) = spawn_contained(&mut command, CREATE_NO_WINDOW).unwrap();
        let ping = within(Duration::from_secs(5), || child_of(child.id(), "PING.EXE"))
            .expect("o ping não subiu");
        (child, job, ping)
    }

    /// Nasce suspenso, mas é solto: roda até o fim, com o código dele.
    #[test]
    fn a_contained_process_runs_to_the_end() {
        let mut command = Command::new("cmd.exe");
        command.args(["/d", "/c", "exit 7"]);
        let (mut child, job) = spawn_contained(&mut command, CREATE_NO_WINDOW).unwrap();
        assert!(job.is_some(), "o processo devia estar no job");
        assert_eq!(child.wait().unwrap().code(), Some(7));
    }

    #[test]
    fn terminating_the_job_takes_the_grandchildren_too() {
        let (mut child, job, ping) = tree();
        job.expect("job").terminate();
        child.wait().unwrap();
        assert!(gone(ping), "o neto sobrou");
    }

    /// Quem criou o job morreu (ou o largou): o que estava dentro vai junto.
    #[test]
    fn closing_the_job_takes_what_was_left() {
        let (mut child, job, ping) = tree();
        drop(job);
        child.wait().unwrap();
        assert!(gone(ping), "o neto sobrou");
    }

    #[test]
    fn a_missing_program_is_an_error() {
        let mut command = Command::new(r"C:\nao\existe\programa.exe");
        assert!(spawn_contained(&mut command, CREATE_NO_WINDOW).is_err());
    }
}
