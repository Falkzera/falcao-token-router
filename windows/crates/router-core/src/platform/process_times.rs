//! O processo existe — e quando começou? (`OpenProcess` + `GetProcessTimes`).
//!
//! É a metade Windows do `ProcessLiveness` do macOS (lá, `kill(pid,0)` +
//! `sysctl`). O Claude Code grava o início do processo no registro da sessão como
//! FILETIME em texto (`procStart`, 100 ns desde 1601-01-01 UTC) — e o valor bateu
//! exatamente com o do processo vivo no spike de 22/09/2026.

use chrono::{DateTime, TimeZone, Utc};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ACCESS_DENIED, FILETIME, HANDLE, STILL_ACTIVE,
};
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

/// De 1601-01-01 a 1970-01-01, em intervalos de 100 ns.
const UNIX_EPOCH_AS_FILETIME: u64 = 116_444_736_000_000_000;

/// FILETIME (100 ns desde 1601, UTC) → data. `None` antes de 1970.
pub fn from_filetime(filetime: u64) -> Option<DateTime<Utc>> {
    let since_unix = filetime.checked_sub(UNIX_EPOCH_AS_FILETIME)?;
    let secs = i64::try_from(since_unix / 10_000_000).ok()?;
    let nanos = u32::try_from((since_unix % 10_000_000) * 100).ok()?;
    Utc.timestamp_opt(secs, nanos).single()
}

/// Data → FILETIME. Datas antes de 1970 viram o início de 1970.
pub fn to_filetime(date: DateTime<Utc>) -> u64 {
    let secs = u64::try_from(date.timestamp()).unwrap_or(0);
    UNIX_EPOCH_AS_FILETIME + secs * 10_000_000 + u64::from(date.timestamp_subsec_nanos()) / 100
}

/// O que o kernel diz de um pid.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProcessState {
    /// Não há processo vivo com esse pid.
    Missing,
    /// Existe e roda; com o instante de início, quando o kernel o deu.
    Running(Option<DateTime<Utc>>),
    /// Existe, mas é de outro usuário/elevado: não dá para ler o início.
    Denied,
}

/// Consulta um pid sem tocar no processo (só leitura de informação limitada).
pub fn probe(pid: u32) -> ProcessState {
    // SAFETY: `OpenProcess` só abre um handle de consulta; nulo = falha.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        // SAFETY: lê o erro da chamada acima, na mesma thread.
        let error = unsafe { GetLastError() };
        // ACCESS_DENIED = existe, de outro dono (o `EPERM` do macOS).
        return if error == ERROR_ACCESS_DENIED {
            ProcessState::Denied
        } else {
            ProcessState::Missing
        };
    }
    let state = if has_exited(handle) {
        // Um processo que já saiu mas cujo objeto alguém ainda segura.
        ProcessState::Missing
    } else {
        ProcessState::Running(creation_time(handle))
    };
    // SAFETY: o handle é nosso e não é mais usado.
    unsafe { CloseHandle(handle) };
    state
}

/// O instante em que o processo subiu, pelo kernel. `None` se não há como saber.
pub fn start_time(pid: u32) -> Option<DateTime<Utc>> {
    match probe(pid) {
        ProcessState::Running(start) => start,
        _ => None,
    }
}

fn has_exited(handle: HANDLE) -> bool {
    let mut code: u32 = 0;
    // SAFETY: handle válido com PROCESS_QUERY_LIMITED_INFORMATION; `code` é nosso.
    let ok = unsafe { GetExitCodeProcess(handle, &mut code) } != 0;
    ok && code != STILL_ACTIVE as u32
}

fn creation_time(handle: HANDLE) -> Option<DateTime<Utc>> {
    let zero = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let (mut creation, mut exit, mut kernel, mut user) = (zero, zero, zero, zero);
    // SAFETY: handle válido; os quatro ponteiros apontam para variáveis nossas.
    let ok =
        unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) } != 0;
    if !ok {
        return None;
    }
    from_filetime((u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime))
}
