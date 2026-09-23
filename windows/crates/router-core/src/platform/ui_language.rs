//! O idioma da interface do Windows, com UM critério para o app e para a CLI (a
//! status line escreve os dias da semana do reset): as duas superfícies nunca
//! discordam.

/// Qualquer português (pt-BR, pt-PT…) usa o catálogo em português, o único além
/// do inglês. O `LANGID` guarda o idioma primário nos 10 bits de baixo.
pub fn is_portuguese(langid: u16) -> bool {
    const LANG_PORTUGUESE: u16 = 0x16;
    langid & 0x3FF == LANG_PORTUGUESE
}

/// A interface do usuário atual está em português?
pub fn portuguese_ui() -> bool {
    // SAFETY: sem argumentos; só lê a configuração do usuário.
    is_portuguese(unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portuguese_of_any_country_counts() {
        assert!(is_portuguese(0x0416)); // pt-BR
        assert!(is_portuguese(0x0816)); // pt-PT
        assert!(!is_portuguese(0x0409)); // en-US
        assert!(!is_portuguese(0x0C0A)); // es-ES
    }
}
