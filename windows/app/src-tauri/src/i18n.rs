//! As strings que o Rust mostra (tooltip e menu da bandeja, título da janela),
//! dos MESMOS catálogos do front (`src/locales/*.json`), embutidos no binário.
//! O formato de placeholder é o do macOS (`%@`, `%d`, `%1$@`, `%%`), com o mesmo
//! preenchimento do `format` de `lib/i18n.ts` — as duas pontas não podem
//! divergir. O `check-strings.mjs` também varre este crate.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::locale::Locale;

const EN: &str = include_str!("../../src/locales/en.json");
const PT_BR: &str = include_str!("../../src/locales/pt-BR.json");

fn catalog(locale: Locale) -> &'static HashMap<String, String> {
    static CATALOGS: OnceLock<(HashMap<String, String>, HashMap<String, String>)> = OnceLock::new();
    let (en, pt) = CATALOGS.get_or_init(|| {
        (
            serde_json::from_str(EN).unwrap_or_default(),
            serde_json::from_str(PT_BR).unwrap_or_default(),
        )
    });
    match locale {
        Locale::En => en,
        Locale::PtBr => pt,
    }
}

/// Um argumento de um modelo: texto (`%@`) ou inteiro (`%d`).
#[derive(Clone, Debug)]
pub enum Arg {
    Text(String),
    Int(i64),
}

impl From<&str> for Arg {
    fn from(value: &str) -> Self {
        Arg::Text(value.to_string())
    }
}

impl From<String> for Arg {
    fn from(value: String) -> Self {
        Arg::Text(value)
    }
}

impl From<i64> for Arg {
    fn from(value: i64) -> Self {
        Arg::Int(value)
    }
}

impl Arg {
    fn render(&self) -> String {
        match self {
            Arg::Text(text) => text.clone(),
            Arg::Int(n) => n.to_string(),
        }
    }
}

/// Preenche um modelo no estilo do macOS. Argumento que falta vira vazio (como
/// no front), nunca pânico.
pub fn format(template: &str, args: &[Arg]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut next = 0usize;
    let mut rest = template;
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        // `%%`
        if let Some(stripped) = after.strip_prefix('%') {
            out.push('%');
            rest = stripped;
            continue;
        }
        // `%N$` opcional
        let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
        let (position, spec) = match after[digits.len()..].strip_prefix('$') {
            Some(spec) if !digits.is_empty() => (digits.parse::<usize>().ok(), spec),
            _ => (None, after),
        };
        match spec.chars().next() {
            Some(kind @ ('@' | 'd')) => {
                let index = match position {
                    Some(p) => p.saturating_sub(1),
                    None => {
                        next += 1;
                        next - 1
                    }
                };
                if let Some(arg) = args.get(index) {
                    match (kind, arg) {
                        ('d', Arg::Text(text)) => out.push_str(text),
                        _ => out.push_str(&arg.render()),
                    }
                }
                rest = &spec[1..];
            }
            _ => {
                // Não é placeholder: o `%` fica como está.
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// O texto de uma chave no idioma dado (o inglês é a base; chave desconhecida
/// volta crua — o `check-strings` existe para isso nunca chegar ao usuário).
pub fn t(locale: Locale, key: &str, args: &[Arg]) -> String {
    let template = catalog(locale)
        .get(key)
        .or_else(|| catalog(Locale::En).get(key))
        .map_or(key, String::as_str);
    if args.is_empty() && !template.contains("%%") {
        template.to_string()
    } else {
        format(template, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Arg {
        Arg::Text(s.to_string())
    }

    /// Os mesmos casos do `format` de `lib/i18n.ts`: as duas pontas preenchem
    /// igual.
    #[test]
    fn fills_like_the_front() {
        assert_eq!(format("resets %@", &[text("13:20")]), "resets 13:20");
        assert_eq!(
            format("%1$@ · in %2$@", &[text("a"), text("b")]),
            "a · in b"
        );
        assert_eq!(
            format("%2$@ antes de %1$@", &[text("a"), text("b")]),
            "b antes de a"
        );
        assert_eq!(
            format("%1$dh %2$dm", &[Arg::Int(1), Arg::Int(12)]),
            "1h 12m"
        );
        assert_eq!(format("%d%% da janela", &[Arg::Int(90)]), "90% da janela");
        assert_eq!(format("sem argumento: %@", &[]), "sem argumento: ");
        assert_eq!(format("100% certo", &[]), "100% certo");
    }

    #[test]
    fn keys_come_from_the_catalog_of_the_locale() {
        assert_eq!(t(Locale::En, "panel.quit", &[]), "Quit");
        assert_eq!(t(Locale::PtBr, "panel.quit", &[]), "Sair");
        assert_eq!(
            t(Locale::PtBr, "chave.que.nao.existe", &[]),
            "chave.que.nao.existe"
        );
    }
}
