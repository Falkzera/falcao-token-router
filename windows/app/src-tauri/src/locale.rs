//! Em qual idioma o app fala: o da interface do Windows. O mesmo para o Rust (a
//! bandeja: tooltip e menu) e para o front, que recebe daqui em vez de adivinhar
//! pelo `navigator.language` — assim as duas superfícies nunca discordam.

use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum Locale {
    #[serde(rename = "en")]
    En,
    #[serde(rename = "pt-BR")]
    PtBr,
}

/// Qualquer português vai para o catálogo pt-BR (o único que há); o resto, para
/// o inglês, a base.
pub fn for_language_id(langid: u16) -> Locale {
    const LANG_PORTUGUESE: u16 = 0x16;
    if langid & 0x3FF == LANG_PORTUGUESE {
        Locale::PtBr
    } else {
        Locale::En
    }
}

/// O idioma da interface do usuário atual.
pub fn system() -> Locale {
    // SAFETY: sem argumentos; só lê a configuração do usuário.
    let langid = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() };
    for_language_id(langid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portuguese_of_any_country_uses_the_pt_br_catalog() {
        assert_eq!(for_language_id(0x0416), Locale::PtBr); // pt-BR
        assert_eq!(for_language_id(0x0816), Locale::PtBr); // pt-PT
        assert_eq!(for_language_id(0x0409), Locale::En); // en-US
        assert_eq!(for_language_id(0x0C0A), Locale::En); // es-ES
    }
}
