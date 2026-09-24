//! O `claude auth login` num ConPTY (≙ a parte de pty do `LoginSession.swift`).
//!
//! Pseudoterminal de propósito: sob um pipe simples o `claude` não imprime o
//! link na hora nem abre o navegador — ele se comporta como no terminal só
//! quando está num. O app nunca vê senha nem token: só lê a saída para achar o
//! link e saber quando terminou; quem grava a credencial é o binário oficial,
//! no perfil isolado da conta.

use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, PtySize};
use router_core::engine::config_dir::ConfigDir;
use router_core::engine::provider_env::ProviderEnv;
use router_core::usage::claude_binary::ClaudeCommand;

use crate::login_output::{LoginEvent, LoginOutput};

/// Larga de propósito: o link do login tem centenas de caracteres e, numa tela
/// estreita, o ConPTY o quebraria em linhas — o texto visível sairia cortado.
const COLUMNS: u16 = 2048;
const ROWS: u16 = 50;

/// A resposta de um terminal ao `ESC[6n`. O `portable-pty` cria o ConPTY com
/// `PSEUDOCONSOLE_INHERIT_CURSOR`, e aí o ConPTY pergunta onde está o cursor e
/// segura a saída até ouvir a resposta (quem responde é o terminal — aqui, nós).
const CURSOR_REPORT: &str = "\x1b[1;1R";

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|p| p.into_inner())
}

/// UTF-8 que chega em pedaços: um caractere pode vir cortado entre duas
/// leituras do pty.
#[derive(Default)]
struct Utf8Stream {
    pending: Vec<u8>,
}

impl Utf8Stream {
    fn push(&mut self, bytes: &[u8]) -> String {
        self.pending.extend_from_slice(bytes);
        let mut text = String::new();
        loop {
            match std::str::from_utf8(&self.pending) {
                Ok(valid) => {
                    text.push_str(valid);
                    self.pending.clear();
                    return text;
                }
                Err(error) => {
                    let valid = error.valid_up_to();
                    text.push_str(std::str::from_utf8(&self.pending[..valid]).unwrap_or_default());
                    match error.error_len() {
                        // Sequência cortada no fim: espera o resto.
                        None => {
                            self.pending.drain(..valid);
                            return text;
                        }
                        Some(bad) => {
                            text.push(char::REPLACEMENT_CHARACTER);
                            self.pending.drain(..valid + bad);
                        }
                    }
                }
            }
        }
    }
}

/// O que a sessão conta a quem a acompanha.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SessionEvent {
    Output(LoginEvent),
    /// O `claude` saiu (com o código) — contado DEPOIS de toda a saída lida.
    Exited(Option<u32>),
}

/// O que rodar: o `claude` achado pelo resolvedor único, a casa da conta, o
/// e-mail esperado (relogin: vai no `--email`, que pré-preenche o login) e o
/// ambiente já pronto (`LoginRequest::environment`).
pub struct LoginRequest {
    pub claude: ClaudeCommand,
    pub home: ConfigDir,
    pub email: Option<String>,
    pub env: Vec<(OsString, OsString)>,
}

impl LoginRequest {
    /// O ambiente do login (≙ macOS): direto — sem proxy nem credencial ou
    /// endpoint alternativo —, sem as variáveis de uma sessão do Claude Code em
    /// volta, com a casa da conta no `CLAUDE_CONFIG_DIR` e um `TERM` de
    /// terminal de verdade.
    pub fn environment(
        base: impl IntoIterator<Item = (OsString, OsString)>,
        home: &ConfigDir,
    ) -> Vec<(OsString, OsString)> {
        let env = ProviderEnv::without_nested_session(ProviderEnv::direct(base));
        let env = ProviderEnv::with_var(
            env,
            "CLAUDE_CONFIG_DIR",
            home.environment_value().map(OsStr::new),
        );
        ProviderEnv::with_var(env, "TERM", Some(OsStr::new("xterm-256color")))
    }
}

/// Um login em andamento.
pub struct LoginSession {
    writer: Mutex<Option<Box<dyn Write + Send>>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    /// O processo saiu e a saída foi toda lida.
    finished: (Mutex<bool>, Condvar),
}

impl LoginSession {
    /// Sobe o `claude auth login` e acompanha a saída numa thread; cada fato
    /// vai para `on_event`, que roda nas threads da sessão.
    pub fn start(
        request: LoginRequest,
        on_event: impl Fn(SessionEvent) + Send + Sync + 'static,
    ) -> std::io::Result<Arc<LoginSession>> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: ROWS,
                cols: COLUMNS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(std::io::Error::other)?;

        let mut command = CommandBuilder::new(&request.claude.program);
        command.args(&request.claude.prefix_args);
        command.args(["auth", "login"]);
        if let Some(email) = &request.email {
            command.args(["--email", email.as_str()]);
        }
        // Só o ambiente pronto: sem o `env_clear`, o `portable-pty` montaria a
        // base com as variáveis do REGISTRO (usuário e sistema), e um proxy ou
        // `CLAUDE_CONFIG_DIR` definidos lá voltariam ao login.
        command.env_clear();
        for (key, value) in &request.env {
            command.env(key, value);
        }
        command.cwd(request.home.path());

        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(std::io::Error::other)?;
        drop(pair.slave);
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(std::io::Error::other)?;
        let writer = pair.master.take_writer().map_err(std::io::Error::other)?;
        let session = Arc::new(LoginSession {
            writer: Mutex::new(Some(writer)),
            killer: Mutex::new(child.clone_killer()),
            finished: (Mutex::new(false), Condvar::new()),
        });
        let on_event = Arc::new(on_event);

        let reading = {
            let (session, on_event) = (session.clone(), on_event.clone());
            thread::Builder::new()
                .name("login-leitura".into())
                .spawn(move || read_output(reader, &session, &*on_event))?
        };
        let master = pair.master;
        let waiter = session.clone();
        thread::Builder::new()
            .name("login-espera".into())
            .spawn(move || {
                let code = child.wait().ok().map(|status| status.exit_code());
                // Fechar o ConPTY é o que faz o leitor ver o fim; e só depois
                // de ele ler TUDO o fim do processo é contado — o "Login
                // successful" impresso antes da saída não pode chegar depois.
                drop(master);
                let _ = reading.join();
                lock(&waiter.writer).take();
                *lock(&waiter.finished.0) = true;
                waiter.finished.1.notify_all();
                on_event(SessionEvent::Exited(code));
            })?;
        Ok(session)
    }

    /// Encerra o `claude` e espera ele sair de fato (até `limit`): quem vai
    /// apagar a casa reservada depois não pode achar arquivo preso por um
    /// processo que ainda está morrendo. Devolve se terminou a tempo.
    pub fn cancel_and_wait(&self, limit: Duration) -> bool {
        self.cancel();
        let (done, signal) = &self.finished;
        let guard = lock(done);
        let (guard, _) = signal
            .wait_timeout_while(guard, limit, |finished| !*finished)
            .unwrap_or_else(|p| p.into_inner());
        *guard
    }

    /// Cola o código que a página do login mostrou (o raro caso em que o
    /// navegador não consegue devolver sozinho): a linha e um Enter.
    pub fn submit_code(&self, code: &str) -> bool {
        let code = code.trim();
        !code.is_empty() && self.write(&format!("{code}\r"))
    }

    /// Encerra o `claude` (o `Exited` chega depois, pela thread da sessão).
    pub fn cancel(&self) {
        let _ = lock(&self.killer).kill();
    }

    fn write(&self, text: &str) -> bool {
        lock(&self.writer).as_mut().is_some_and(|writer| {
            writer.write_all(text.as_bytes()).is_ok() && writer.flush().is_ok()
        })
    }
}

/// Lê o pty até o fim: responde ao pedido de posição do cursor, dá o Enter que
/// o sucesso pedir e passa adiante cada fato do login.
fn read_output(
    mut reader: Box<dyn Read + Send>,
    session: &LoginSession,
    on_event: &(dyn Fn(SessionEvent) + Send + Sync),
) {
    let mut output = LoginOutput::new();
    let mut utf8 = Utf8Stream::default();
    let mut buffer = [0u8; 4096];
    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let events = output.feed(&utf8.push(&buffer[..read]));
        for _ in 0..output.take_cursor_queries() {
            session.write(CURSOR_REPORT);
        }
        for event in events {
            if event == LoginEvent::NeedsEnter {
                session.write("\r");
            }
            on_event(SessionEvent::Output(event));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::mpsc::{channel, Receiver};
    use std::time::Duration;

    /// O `fake-claude` do workspace (o `cargo test --workspace` o compila).
    fn fake_claude() -> ClaudeCommand {
        let exe = std::env::current_exe().unwrap();
        let program = exe
            .parent()
            .and_then(|deps| deps.parent())
            .unwrap()
            .join("fake-claude.exe");
        assert!(
            program.is_file(),
            "fake-claude não foi compilado — rode `cargo test --workspace` (ou scripts\\test.ps1)"
        );
        ClaudeCommand {
            program,
            prefix_args: Vec::new(),
        }
    }

    struct Run {
        _tmp: tempfile::TempDir,
        home: ConfigDir,
        record: PathBuf,
        session: Arc<LoginSession>,
        events: Receiver<SessionEvent>,
    }

    /// Um login do `fake-claude` numa casa temporária. O ambiente de base é o
    /// deste processo (que pode ser uma sessão do Claude Code, com um
    /// `CLAUDE_CONFIG_DIR` real — o que o login precisa trocar) mais `extra`.
    fn start(mode: &str, email: Option<&str>, extra: &[(&str, &str)]) -> Run {
        let tmp = tempfile::tempdir().unwrap();
        let home = ConfigDir::dedicated(tmp.path().join("conta").to_string_lossy().into_owned());
        std::fs::create_dir_all(home.path()).unwrap();
        let record = tmp.path().join("fake-claude.jsonl");
        let mut base: Vec<(OsString, OsString)> = std::env::vars_os().collect();
        base.push(("FAKE_CLAUDE_LOGIN".into(), mode.into()));
        base.push(("FAKE_CLAUDE_RECORD".into(), record.clone().into()));
        for (k, v) in extra {
            base.push(((*k).into(), (*v).into()));
        }
        let (tx, events) = channel();
        let session = LoginSession::start(
            LoginRequest {
                claude: fake_claude(),
                home: home.clone(),
                email: email.map(String::from),
                env: LoginRequest::environment(base, &home),
            },
            move |event| {
                let _ = tx.send(event);
            },
        )
        .unwrap();
        Run {
            _tmp: tmp,
            home,
            record,
            session,
            events,
        }
    }

    impl Run {
        fn next(&self) -> SessionEvent {
            self.events
                .recv_timeout(Duration::from_secs(20))
                .expect("a sessão parou de falar")
        }

        /// Os eventos até o fim do processo (inclusive).
        fn until_exit(&self) -> Vec<SessionEvent> {
            let mut all = Vec::new();
            loop {
                let event = self.next();
                let done = matches!(event, SessionEvent::Exited(_));
                all.push(event);
                if done {
                    return all;
                }
            }
        }

        fn record(&self) -> serde_json::Value {
            let text = std::fs::read_to_string(&self.record).unwrap();
            serde_json::from_str(text.lines().next().unwrap()).unwrap()
        }
    }

    fn is_url(event: &SessionEvent) -> bool {
        matches!(event, SessionEvent::Output(LoginEvent::Url(url)) if url.starts_with("https://claude.com/cai/oauth/authorize?"))
    }

    /// O caminho feliz, pelo ConPTY de verdade: o link, o sucesso e SÓ ENTÃO
    /// o fim do processo — e a credencial e a identidade na casa da conta.
    #[test]
    fn a_login_reports_the_link_then_success_then_the_exit() {
        let run = start("ok:conta1@exemplo.com", None, &[]);
        let events = run.until_exit();
        assert!(is_url(&events[0]), "{events:?}");
        assert_eq!(
            &events[1..],
            &[
                SessionEvent::Output(LoginEvent::Success),
                SessionEvent::Exited(Some(0))
            ]
        );
        assert!(run.home.path().join(".credentials.json").is_file());
        assert!(run.home.path().join(".claude.json").is_file());
    }

    /// O código colado chega ao `claude` como uma linha digitada; um código
    /// sem a forma `código#state` é recusado e o login continua esperando.
    #[test]
    fn a_pasted_code_is_typed_into_the_terminal() {
        let run = start("code:conta1@exemplo.com", None, &[]);
        assert!(is_url(&run.next()));
        assert!(run.session.submit_code("incompleto"));
        assert_eq!(run.next(), SessionEvent::Output(LoginEvent::InvalidCode));
        assert!(run.session.submit_code("  codigo#estado  "));
        assert_eq!(
            run.until_exit(),
            vec![
                SessionEvent::Output(LoginEvent::Success),
                SessionEvent::Exited(Some(0))
            ]
        );
    }

    #[test]
    fn a_refused_login_carries_the_reason() {
        let run = start("fail:Request failed with status code 403", None, &[]);
        let events = run.until_exit();
        assert!(is_url(&events[0]), "{events:?}");
        assert_eq!(
            &events[1..],
            &[
                SessionEvent::Output(LoginEvent::Failed(
                    "Request failed with status code 403".into()
                )),
                SessionEvent::Exited(Some(1))
            ]
        );
    }

    /// Cancelar encerra o `claude` (que esperaria para sempre) e ESPERA o fim:
    /// logo depois, a casa reservada pode ser apagada (nenhum arquivo preso).
    #[test]
    fn cancel_ends_the_process_and_frees_the_home() {
        let run = start("hang", None, &[]);
        assert!(is_url(&run.next()));
        assert!(run.session.cancel_and_wait(Duration::from_secs(10)));
        assert!(matches!(
            run.until_exit().last(),
            Some(SessionEvent::Exited(_))
        ));
        std::fs::remove_dir_all(run.home.path()).expect("a casa ficou presa");
    }

    /// Relogin: o e-mail vai no `--email`. O ambiente é o do login: a casa no
    /// `CLAUDE_CONFIG_DIR` (não o da sessão em volta), sem proxy, sem
    /// credencial alternativa, sem as variáveis de sessão do Claude Code.
    #[test]
    fn the_relogin_passes_the_email_and_the_environment_is_direct() {
        let run = start(
            "ok:conta1@exemplo.com",
            Some("conta1@exemplo.com"),
            &[
                ("HTTPS_PROXY", "http://127.0.0.1:9"),
                ("ANTHROPIC_API_KEY", "falsa"),
                (
                    "CLAUDE_SECURESTORAGE_CONFIG_DIR",
                    "C:\\Users\\exemplo\\outro",
                ),
                ("CLAUDE_CODE_OAUTH_TOKEN", "falso"),
                ("CLAUDECODE", "1"),
                ("CLAUDE_CODE_ENTRYPOINT", "cli"),
            ],
        );
        let events = run.until_exit();
        let url = match &events[0] {
            SessionEvent::Output(LoginEvent::Url(url)) => url.clone(),
            other => panic!("{other:?}"),
        };
        assert!(url.ends_with("&login_hint=conta1@exemplo.com"), "{url}");
        let record = run.record();
        assert_eq!(
            record["args"],
            serde_json::json!(["auth", "login", "--email", "conta1@exemplo.com"])
        );
        assert_eq!(record["claudeConfigDir"], serde_json::json!(run.home.raw));
        assert_eq!(record["present"], serde_json::json!([]), "{record}");
    }
}
