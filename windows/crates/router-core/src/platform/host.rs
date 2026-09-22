//! Os nomes desta máquina, para ler o `pidDomain` dos registros de sessão.
//!
//! O Claude Code escreve `pidDomain: "win32:<host>"` com o nome DNS da máquina
//! em minúsculas (visto no spike de 22/09/2026) — que pode diferir do
//! `%COMPUTERNAME%` (NetBIOS, maiúsculo, até 15 caracteres). Os dois entram na
//! lista, e quem compara ignora a caixa.

use windows_sys::Win32::System::SystemInformation::{ComputerNameDnsHostname, GetComputerNameExW};

/// Os nomes desta máquina (DNS e NetBIOS), sem repetição. Nunca vazio.
pub fn local_host_names() -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    if let Some(dns) = dns_host_name() {
        names.push(dns);
    }
    if let Ok(netbios) = std::env::var("COMPUTERNAME") {
        if !netbios.is_empty() && !names.iter().any(|n| n.eq_ignore_ascii_case(&netbios)) {
            names.push(netbios);
        }
    }
    if names.is_empty() {
        names.push("localhost".to_string());
    }
    names
}

fn dns_host_name() -> Option<String> {
    let mut size: u32 = 0;
    // SAFETY: com buffer nulo a chamada só informa o tamanho necessário.
    unsafe { GetComputerNameExW(ComputerNameDnsHostname, std::ptr::null_mut(), &mut size) };
    if size == 0 {
        return None;
    }
    let mut buffer = vec![0u16; size as usize];
    // SAFETY: `buffer` tem `size` posições, como a chamada anterior pediu.
    let ok =
        unsafe { GetComputerNameExW(ComputerNameDnsHostname, buffer.as_mut_ptr(), &mut size) } != 0;
    if !ok || size == 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buffer[..size as usize]))
}
