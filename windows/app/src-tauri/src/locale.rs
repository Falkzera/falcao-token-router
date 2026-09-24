//! Em qual idioma o app fala: o da interface do Windows. O mesmo para o Rust (a
//! bandeja: tooltip e menu) e para o front, que recebe daqui em vez de adivinhar
//! pelo `navigator.language` — assim as duas superfícies nunca discordam.

use router_core::platform::ui_language;
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum Locale {
    #[serde(rename = "en")]
    En,
    #[serde(rename = "pt-BR")]
    PtBr,
}

/// O idioma da interface do usuário atual: qualquer português vai para o
/// catálogo pt-BR (o único que há); o resto, para o inglês, a base. O critério
/// mora no núcleo (`ui_language`): a status line da CLI usa o mesmo.
pub fn system() -> Locale {
    from_portuguese(ui_language::portuguese_ui())
}

fn from_portuguese(portuguese: bool) -> Locale {
    if portuguese {
        Locale::PtBr
    } else {
        Locale::En
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portuguese_of_any_country_uses_the_pt_br_catalog() {
        let locale = |langid| from_portuguese(ui_language::is_portuguese(langid));
        assert_eq!(locale(0x0416), Locale::PtBr); // pt-BR
        assert_eq!(locale(0x0816), Locale::PtBr); // pt-PT
        assert_eq!(locale(0x0409), Locale::En); // en-US
        assert_eq!(locale(0x0C0A), Locale::En); // es-ES
    }
}
