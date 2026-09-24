//! O Ctrl+C no `router launch`.
//!
//! No macOS o `launch` faz `exec` e deixa de existir: o `claude` herda o PID e o
//! terminal inteiro. No Windows não há `exec` — o `router` sobe o `claude` como
//! filho e espera, para repassar o código de saída. Um Ctrl+C no console vai
//! para TODOS os processos presos a ele; se o `router` morresse, o shell voltaria
//! ao prompt com o `claude` ainda rodando por baixo.
//!
//! Então o `router` ignora Ctrl+C/Ctrl+Break enquanto o filho roda — com um
//! handler PRÓPRIO que devolve TRUE. O atalho `SetConsoleCtrlHandler(NULL, TRUE)`
//! não serve: ele liga um atributo do processo que é HERDADO pelos filhos, e o
//! `claude` passaria a ignorar Ctrl+C também (não daria para interromper uma
//! resposta). Handler registrado não é herdado.

use windows_sys::core::BOOL;
use windows_sys::Win32::System::Console::{SetConsoleCtrlHandler, CTRL_BREAK_EVENT, CTRL_C_EVENT};

unsafe extern "system" fn ignore_interrupts(ctrl_type: u32) -> BOOL {
    // Fechar a janela, logoff e desligamento seguem o caminho normal.
    BOOL::from(ctrl_type == CTRL_C_EVENT || ctrl_type == CTRL_BREAK_EVENT)
}

/// Faz ESTE processo sobreviver a Ctrl+C/Ctrl+Break (o filho continua
/// recebendo e tratando os dele). Devolve `false` se o Windows recusou.
pub fn ignore_interrupts_in_this_process() -> bool {
    // SAFETY: registra uma função `extern "system"` com a assinatura exata de
    // `PHANDLER_ROUTINE`, que vive pelo programa inteiro.
    unsafe { SetConsoleCtrlHandler(Some(ignore_interrupts), 1) != 0 }
}
