//! A trava do motor entre o app e a CLI — novo no porte.
//!
//! O macOS não tem trava nenhuma: o laço de rotação do app e um `router launch`
//! no terminal podem ativar/espelhar ao mesmo tempo, e duas escritas cruzadas na
//! mesma credencial são exatamente o tipo de corrida que mata token. Aqui toda
//! ação que escreve credencial (ativar, espelhar, empurrar pós-relogin) passa por
//! um mutex nomeado do Windows, `Local\com.synqo.falcao-router.engine.<hash>`.
//!
//! O nome deriva da base (`ROUTER_APP_SUPPORT` respeitado): app e CLI apontando
//! para a mesma base disputam a mesma trava; bases diferentes (testes) não se
//! esbarram. A trava é segurança a mais, não um portão: quem não a consegue no
//! prazo segue assim mesmo — travar o `claude` do usuário por causa dela seria
//! pior do que a corrida que ela evita.

use std::path::Path;
use std::time::Duration;

use crate::platform::named_mutex::{self, NamedMutexGuard};

pub struct EngineLock;

impl EngineLock {
    /// Quanto esperar por outro processo. As seções travadas são escritas de
    /// arquivo (milissegundos); 5 s só se esgotam com alguém travado de verdade.
    pub const WAIT: Duration = Duration::from_secs(5);

    /// O nome do mutex para uma base.
    pub fn name_for(base: &Path) -> String {
        let normalized = base
            .to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase();
        format!(
            "Local\\com.synqo.falcao-router.engine.{:016x}",
            fnv1a64(normalized.as_bytes())
        )
    }

    /// Pega a trava da base, esperando até `timeout`. `None` = seguir sem ela.
    pub fn acquire(base: &Path, timeout: Duration) -> Option<NamedMutexGuard> {
        named_mutex::acquire(&Self::name_for(base), timeout)
            .ok()
            .flatten()
    }
}

/// FNV-1a de 64 bits: estável entre versões e processos (o `DefaultHasher` do
/// Rust não promete isso), e o app e a CLI precisam chegar ao MESMO nome.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
