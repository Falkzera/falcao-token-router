//! O que a saída do `claude auth login` diz, lida através de um ConPTY.
//!
//! O ConPTY não repassa os bytes do filho: ele os desenha numa tela e manda para
//! nós as sequências VT que redesenham essa tela (cursor, cor, título, apagar
//! linha). Daqui sai só o texto — em fluxo, porque uma sequência pode chegar
//! cortada entre dois pedaços — e, dele, os fatos do login.
//!
//! O que o `claude` 2.1.280 imprime (lido no JS do binário em 23/09/2026):
//! `Opening browser to sign in…`, `If the browser didn't open, visit: <URL>`
//! (a URL num hyperlink OSC 8 quando a saída é terminal), `Paste code here if
//! prompted > ` e, no fim, `Login successful.` (sai com 0 sozinho) ou
//! `Login failed: <motivo>` (sai com 1). Um código colado sem a forma
//! `código#state` dá `Invalid code. …` e o login continua esperando.
//! `Login interrupted` e o "Press Enter" depois do sucesso são da TUI (`/login`)
//! — ficam reconhecidos, para uma versão que os mostre no `auth login`.

/// Um fato da saída do login, emitido uma vez.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LoginEvent {
    /// O link do login oficial (o `claude` já abriu o navegador nele).
    Url(String),
    /// O código colado não tinha a forma `código#state` (a cada vez).
    InvalidCode,
    /// `Login successful`: a credencial foi gravada na casa.
    Success,
    /// Depois do sucesso o `claude` pede um Enter.
    NeedsEnter,
    /// `Login failed: …` (o motivo) ou `Login interrupted`.
    Failed(String),
}

use std::sync::OnceLock;

use regex::Regex;

/// O login só vale nestes hosts (os mesmos que `settings::allowed_url` abre).
const OFFICIAL_HOSTS: &[&str] = &["https://claude.com/", "https://platform.claude.com/"];

/// Teto do texto guardado: a saída do login cabe em poucas linhas; um filho
/// que imprime sem parar não pode crescer a memória do app sem limite.
const TEXT_LIMIT: usize = 256 * 1024;

/// Teto de uma sequência de escape à espera do resto (OSC sem terminador).
const PENDING_LIMIT: usize = 16 * 1024;

/// Lê a saída em pedaços e diz o que aconteceu.
#[derive(Default)]
pub struct LoginOutput {
    /// O começo de uma sequência de escape que chegou cortada.
    pending: String,
    /// O texto visível até aqui (sem sequências; quebras de linha como `\n`).
    text: String,
    /// O alvo do hyperlink OSC 8 — inteiro mesmo se o texto visível quebrar.
    hyperlink: Option<String>,
    url_sent: bool,
    success_sent: bool,
    enter_sent: bool,
    failure_sent: bool,
    invalid_seen: usize,
    cursor_queries: usize,
}

/// O que uma sequência de escape vira no texto.
enum Action {
    Nothing,
    Newline,
    Spaces(usize),
    CursorQuery,
    Hyperlink(String),
}

enum Escape {
    /// Faltam caracteres: espera o próximo pedaço.
    Incomplete,
    /// A sequência ocupa `len` caracteres.
    Done { len: usize, action: Action },
}

fn is_official(url: &str) -> bool {
    OFFICIAL_HOSTS.iter().any(|host| url.starts_with(host))
}

/// O primeiro parâmetro numérico de um CSI (`3;1` → 3), com o padrão do VT.
fn first_param(params: &str, default: usize) -> usize {
    params
        .split(';')
        .next()
        .and_then(|p| p.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(default)
}

/// Uma sequência que começa em `chars[0] == ESC` (ECMA-48: CSI, as de texto
/// — OSC/DCS/SOS/PM/APC, terminadas por BEL ou ST — e as de dois caracteres).
fn parse_escape(chars: &[char]) -> Escape {
    let Some(&kind) = chars.get(1) else {
        return Escape::Incomplete;
    };
    match kind {
        '[' => {
            let mut j = 2;
            while chars.get(j).is_some_and(|c| ('\x30'..='\x3f').contains(c)) {
                j += 1;
            }
            while chars.get(j).is_some_and(|c| ('\x20'..='\x2f').contains(c)) {
                j += 1;
            }
            let Some(&last) = chars.get(j) else {
                return Escape::Incomplete;
            };
            if !('\x40'..='\x7e').contains(&last) {
                // Malformada: descarta o que veio e segue do caractere estranho.
                return Escape::Done {
                    len: j,
                    action: Action::Nothing,
                };
            }
            let params: String = chars[2..j].iter().collect();
            let action = match last {
                'n' if params == "6" => Action::CursorQuery,
                'C' => Action::Spaces(first_param(&params, 1).min(256)),
                'H' | 'f' => Action::Newline,
                _ => Action::Nothing,
            };
            Escape::Done { len: j + 1, action }
        }
        ']' | 'P' | 'X' | '^' | '_' => {
            let mut j = 2;
            let (end, len) = loop {
                match chars.get(j) {
                    None => return Escape::Incomplete,
                    Some('\x07') => break (j, j + 1),
                    Some('\x1b') => match chars.get(j + 1) {
                        None => return Escape::Incomplete,
                        Some('\\') => break (j, j + 2),
                        // Outra sequência começou sem terminar esta: esta acaba aqui.
                        Some(_) => break (j, j),
                    },
                    Some(_) => j += 1,
                }
            };
            let body: String = chars[2..end].iter().collect();
            let action = match (kind, body.strip_prefix("8;")) {
                // `ESC ] 8 ; params ; URI` — o URI vazio fecha o hyperlink.
                (']', Some(rest)) => rest
                    .split_once(';')
                    .map(|(_, uri)| uri)
                    .filter(|uri| !uri.is_empty())
                    .map_or(Action::Nothing, |uri| Action::Hyperlink(uri.to_string())),
                _ => Action::Nothing,
            };
            Escape::Done { len, action }
        }
        c if ('\x20'..='\x2f').contains(&c) => {
            let mut j = 2;
            while chars.get(j).is_some_and(|c| ('\x20'..='\x2f').contains(c)) {
                j += 1;
            }
            if j >= chars.len() {
                return Escape::Incomplete;
            }
            Escape::Done {
                len: j + 1,
                action: Action::Nothing,
            }
        }
        _ => Escape::Done {
            len: 2,
            action: Action::Nothing,
        },
    }
}

/// O link do texto visível, só quando COMPLETO: começa num limite (início de
/// linha ou espaço — um link da Anthropic dentro da query de outro não conta)
/// e termina num branco.
fn visible_url(text: &str) -> Option<String> {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN
        .get_or_init(|| {
            Regex::new(r"(?m)(?:^|\s)(https://(?:platform\.)?claude\.com/\S+)\s")
                .expect("regex do link")
        })
        .captures(text)
        .map(|c| c[1].to_string())
}

/// `Login failed: <motivo>` com a linha inteira, ou `Login interrupted`.
fn failure(text: &str) -> Option<String> {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    let failed = PATTERN.get_or_init(|| Regex::new(r"Login failed:([^\n]*)\n").expect("regex"));
    if let Some(c) = failed.captures(text) {
        let reason = c[1].trim();
        return Some(if reason.is_empty() {
            "Login failed".to_string()
        } else {
            reason.to_string()
        });
    }
    text.contains("Login interrupted")
        .then(|| "Login interrupted".to_string())
}

impl LoginOutput {
    pub fn new() -> Self {
        Self::default()
    }

    /// Um pedaço da saída; devolve os fatos novos.
    pub fn feed(&mut self, chunk: &str) -> Vec<LoginEvent> {
        self.pending.push_str(chunk);
        self.consume();
        self.trim();
        self.events()
    }

    /// Quantas vezes o terminal pediu a posição do cursor (`ESC[6n`) desde a
    /// última consulta. Quem dirige o pty responde a cada uma.
    pub fn take_cursor_queries(&mut self) -> usize {
        std::mem::take(&mut self.cursor_queries)
    }

    /// O texto limpo até aqui.
    #[cfg(test)]
    pub fn text(&self) -> &str {
        &self.text
    }

    fn consume(&mut self) {
        let input = std::mem::take(&mut self.pending);
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            match chars[i] {
                '\x1b' => match parse_escape(&chars[i..]) {
                    Escape::Incomplete => {
                        if chars.len() - i <= PENDING_LIMIT {
                            self.pending = chars[i..].iter().collect();
                        }
                        return;
                    }
                    Escape::Done { len, action } => {
                        self.apply(action);
                        i += len.max(1);
                    }
                },
                '\n' => {
                    self.text.push('\n');
                    i += 1;
                }
                '\t' => {
                    self.text.push(' ');
                    i += 1;
                }
                // `\r` (o `\n` que o segue já quebra a linha), BEL, BS e o resto.
                c if c.is_control() => i += 1,
                c => {
                    self.text.push(c);
                    i += 1;
                }
            }
        }
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::Nothing => {}
            Action::Newline => self.text.push('\n'),
            Action::Spaces(n) => self.text.extend(std::iter::repeat_n(' ', n)),
            Action::CursorQuery => self.cursor_queries += 1,
            Action::Hyperlink(uri) => {
                if self.hyperlink.is_none() && is_official(&uri) {
                    self.hyperlink = Some(uri);
                }
            }
        }
    }

    fn trim(&mut self) {
        if self.text.len() > TEXT_LIMIT {
            let mut cut = self.text.len() - TEXT_LIMIT / 2;
            while !self.text.is_char_boundary(cut) {
                cut += 1;
            }
            self.text.drain(..cut);
            self.invalid_seen = self.text.matches("Invalid code").count();
        }
    }

    fn events(&mut self) -> Vec<LoginEvent> {
        let mut events = Vec::new();
        if !self.url_sent {
            if let Some(url) = self.hyperlink.clone().or_else(|| visible_url(&self.text)) {
                self.url_sent = true;
                events.push(LoginEvent::Url(url));
            }
        }
        let invalid = self.text.matches("Invalid code").count();
        for _ in self.invalid_seen..invalid {
            events.push(LoginEvent::InvalidCode);
        }
        self.invalid_seen = invalid;
        if !self.success_sent && !self.failure_sent && self.text.contains("Login successful") {
            self.success_sent = true;
            events.push(LoginEvent::Success);
        }
        if self.success_sent && !self.enter_sent {
            let after = self
                .text
                .find("Login successful")
                .map_or("", |i| &self.text[i..]);
            if after.contains("Press Enter") {
                self.enter_sent = true;
                events.push(LoginEvent::NeedsEnter);
            }
        }
        if !self.success_sent && !self.failure_sent {
            if let Some(reason) = failure(&self.text) {
                self.failure_sent = true;
                events.push(LoginEvent::Failed(reason));
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &str = "https://claude.com/cai/oauth/authorize?code=true&client_id=00000000-0000-4000-8000-000000000000&response_type=code&redirect_uri=http%3A%2F%2Flocalhost%3A54545%2Fcallback&scope=user%3Ainference&state=exemplo";

    /// O que um ConPTY do Windows 11 manda para um `claude auth login`: o
    /// pedido da posição do cursor, modos, título da janela, cor e o link num
    /// hyperlink OSC 8 (os textos são os do JS do 2.1.280).
    fn transcript() -> String {
        format!(
            "\x1b[6n\x1b[?9001h\x1b[?1004h\x1b[?25l\x1b[2J\x1b[m\x1b[H\
             \x1b]0;C:\\Users\\exemplo\\.local\\bin\\claude.exe\x07\x1b[?25h\
             Opening browser to sign in\u{2026}\r\n\
             If the browser didn't open, visit: \x1b]8;;{URL}\x1b\\\x1b[94m{URL}\x1b[39m\x1b]8;;\x1b\\\r\n\
             Paste code here if prompted > "
        )
    }

    fn no_control(text: &str) -> bool {
        !text.chars().any(|c| c.is_control() && c != '\n')
    }

    #[test]
    fn the_link_comes_out_clean_from_a_conpty_transcript() {
        let mut out = LoginOutput::new();
        assert_eq!(
            out.feed(&transcript()),
            vec![LoginEvent::Url(URL.to_string())]
        );
        let text = out.text();
        assert!(
            text.contains("Opening browser to sign in\u{2026}"),
            "{text:?}"
        );
        assert!(text.contains(&format!("visit: {URL}")), "{text:?}");
        assert!(text.contains("Paste code here if prompted > "), "{text:?}");
        assert!(
            !text.contains("claude.exe"),
            "o título da janela não é texto"
        );
        assert!(no_control(text), "{text:?}");
    }

    /// O fluxo que o ConPTY do Windows 11 entregou de verdade (23/09/2026, com o
    /// `fake-claude` imprimindo o que o 2.1.280 imprime; o caminho do título
    /// anonimizado): ele RE-EMITE o hyperlink como `ESC]8;id=<n>;URL ESC\`, e o
    /// sucesso cai na mesma linha do prompt do código.
    #[test]
    fn a_recorded_conpty_stream() {
        let link = "https://claude.com/cai/oauth/authorize?code=true&client_id=fake&state=fake";
        let raw = format!(
            "\x1b[6n\x1b[?9001h\x1b[?1004h\x1b[m\x1b]0;C:\\Users\\exemplo\\.local\\bin\\claude.exe\x07\x1b[?25h\
             Opening browser to sign in\u{2026}\r\n\
             If the browser didn't open, visit: \x1b[94m\x1b]8;id=37268-1;{link}\x1b\\{link}\x1b[m\x1b]8;;\x1b\\\r\n\
             Paste code here if prompted > Login successful.\r\n\
             \x1b[?9001l\x1b[?1004l"
        );
        let mut out = LoginOutput::new();
        assert_eq!(
            out.feed(&raw),
            vec![LoginEvent::Url(link.to_string()), LoginEvent::Success]
        );
        assert_eq!(out.take_cursor_queries(), 1);
        assert!(no_control(out.text()), "{:?}", out.text());
    }

    /// O ConPTY entrega em pedaços do tamanho que quiser: uma sequência cortada
    /// ao meio não pode vazar como texto, nem o link sair duas vezes.
    #[test]
    fn sequences_cut_between_chunks_do_not_leak() {
        let mut out = LoginOutput::new();
        let mut events = Vec::new();
        for ch in transcript().chars() {
            events.extend(out.feed(&ch.to_string()));
        }
        assert_eq!(events, vec![LoginEvent::Url(URL.to_string())]);
        assert!(no_control(out.text()), "{:?}", out.text());
        assert!(out.text().contains(&format!("visit: {URL}")));
    }

    /// Sem hyperlink, o link só conta quando termina (espaço ou fim de linha
    /// depois dele) — um link pela metade abriria a página errada.
    #[test]
    fn the_link_waits_until_it_is_complete() {
        let mut out = LoginOutput::new();
        assert_eq!(out.feed("visit: https://claude.com/cai/oauth/auth"), vec![]);
        assert_eq!(
            out.feed("orize?x=1\r\n"),
            vec![LoginEvent::Url(
                "https://claude.com/cai/oauth/authorize?x=1".into()
            )]
        );
    }

    /// Os dois hosts do login oficial; nenhum outro — nem um que só começa
    /// igual, nem um link da Anthropic escondido dentro de outro.
    #[test]
    fn only_the_official_hosts_count() {
        let mut out = LoginOutput::new();
        assert_eq!(
            out.feed("visit: https://platform.claude.com/oauth/authorize?x=1\r\n"),
            vec![LoginEvent::Url(
                "https://platform.claude.com/oauth/authorize?x=1".into()
            )]
        );
        for text in [
            "visit: https://claude.com.exemplo.com/oauth\r\n",
            "visit: http://claude.com/oauth\r\n",
            "visit: https://exemplo.com/?next=https://claude.com/oauth\r\n",
        ] {
            assert_eq!(LoginOutput::new().feed(text), vec![], "{text}");
        }
    }

    /// Numa tela estreita o texto visível quebra; o link do hyperlink (OSC 8)
    /// chega inteiro e é o que vale.
    #[test]
    fn the_hyperlink_wins_over_wrapped_visible_text() {
        let (head, tail) = URL.split_at(60);
        let mut out = LoginOutput::new();
        let events = out.feed(&format!(
            "visit: \x1b]8;;{URL}\x07{head}\r\n{tail}\x1b]8;;\x07\r\n"
        ));
        assert_eq!(events, vec![LoginEvent::Url(URL.to_string())]);
    }

    /// O ConPTY pode redesenhar a tela: o mesmo texto de novo não é um
    /// segundo sucesso.
    #[test]
    fn success_is_reported_once_even_when_redrawn() {
        let mut out = LoginOutput::new();
        assert_eq!(
            out.feed("Paste code here if prompted > Login successful.\r\n"),
            vec![LoginEvent::Success]
        );
        assert_eq!(
            out.feed("\x1b[H\x1b[2KPaste code here if prompted > Login successful.\r\n"),
            vec![]
        );
    }

    #[test]
    fn a_press_enter_after_success_is_asked_for() {
        let mut out = LoginOutput::new();
        assert_eq!(
            out.feed("Login successful. Press Enter to continue\u{2026}"),
            vec![LoginEvent::Success, LoginEvent::NeedsEnter]
        );
    }

    /// O motivo vem inteiro: espera o fim da linha.
    #[test]
    fn a_failure_carries_the_whole_reason() {
        let mut out = LoginOutput::new();
        assert_eq!(out.feed("\r\nLogin failed: Request fa"), vec![]);
        assert_eq!(
            out.feed("iled with status code 403\r\n"),
            vec![LoginEvent::Failed(
                "Request failed with status code 403".into()
            )]
        );
        assert_eq!(
            LoginOutput::new().feed("Login interrupted\r\n"),
            vec![LoginEvent::Failed("Login interrupted".into())]
        );
    }

    /// Cada código inválido é um aviso (o usuário pode colar de novo).
    #[test]
    fn each_invalid_code_is_reported() {
        let mut out = LoginOutput::new();
        let line = "Invalid code. Please make sure the full code was copied.\r\n";
        assert_eq!(out.feed(line), vec![LoginEvent::InvalidCode]);
        assert_eq!(out.feed("Paste code here if prompted > "), vec![]);
        assert_eq!(out.feed(line), vec![LoginEvent::InvalidCode]);
    }

    /// Com `PSEUDOCONSOLE_INHERIT_CURSOR` (o `portable-pty` o usa) o ConPTY
    /// pergunta a posição do cursor e segura a saída até a resposta.
    #[test]
    fn the_cursor_position_request_is_counted() {
        let mut out = LoginOutput::new();
        out.feed("\x1b[6");
        assert_eq!(out.take_cursor_queries(), 0, "sequência ainda incompleta");
        out.feed("nOpening");
        assert_eq!(out.take_cursor_queries(), 1);
        assert_eq!(out.take_cursor_queries(), 0);
    }

    /// O ConPTY pula brancos com "cursor para a frente" e troca de linha
    /// posicionando o cursor — no texto, espaço e quebra de linha.
    #[test]
    fn cursor_moves_become_spaces_and_line_breaks() {
        let mut out = LoginOutput::new();
        let events =
            out.feed("Paste code here if prompted >\x1b[1C\x1b[3;1HLogin\x1b[1Csuccessful.");
        assert_eq!(events, vec![LoginEvent::Success]);
        assert!(
            out.text().ends_with("prompted > \nLogin successful."),
            "{:?}",
            out.text()
        );
    }
}
