//! Mutex nomeado do Windows (`CreateMutexW`) — uma trava entre PROCESSOS.
//!
//! Serve para o app e a CLI não mexerem na mesma credencial ao mesmo tempo (o
//! laço de rotação do app e um `router launch` no terminal, por exemplo). O dono
//! é a thread que pegou; por isso a guarda não é `Send`. Um dono que morreu sem
//! soltar deixa o mutex "abandonado", e quem espera o recebe mesmo assim.

use std::io;
use std::marker::PhantomData;
use std::time::Duration;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

/// A posse do mutex. Solta ao sair de escopo, na mesma thread que pegou.
pub struct NamedMutexGuard {
    handle: HANDLE,
    // `*const ()` tira o `Send`: soltar noutra thread falharia em silêncio.
    _same_thread: PhantomData<*const ()>,
}

/// Espera até `timeout` pela posse do mutex `name`. `Ok(None)` = não deu tempo.
pub fn acquire(name: &str, timeout: Duration) -> io::Result<Option<NamedMutexGuard>> {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: `wide` é uma string UTF-16 terminada em zero que vive até o fim da
    // chamada; atributos nulos = padrão; `0` = não pegar a posse na criação.
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, wide.as_ptr()) };
    if handle.is_null() {
        return Err(io::Error::last_os_error());
    }
    let millis = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX - 1);
    // SAFETY: `handle` é um handle de mutex válido, recém-aberto.
    let waited = unsafe { WaitForSingleObject(handle, millis) };
    if waited == WAIT_OBJECT_0 || waited == WAIT_ABANDONED {
        Ok(Some(NamedMutexGuard {
            handle,
            _same_thread: PhantomData,
        }))
    } else {
        // SAFETY: o handle é nosso e não será mais usado.
        unsafe { CloseHandle(handle) };
        Ok(None)
    }
}

impl Drop for NamedMutexGuard {
    fn drop(&mut self) {
        // SAFETY: esta thread detém a posse (garantido pelo `!Send`) e o handle
        // é nosso; depois de soltar, fecha.
        unsafe {
            ReleaseMutex(self.handle);
            CloseHandle(self.handle);
        }
    }
}
