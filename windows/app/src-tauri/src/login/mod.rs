//! O login dentro do app (≙ `LoginSheet` + `PendingLogin` do macOS): a sessão
//! ConPTY, as fases que a tela mostra e o desfecho conferido NO DISCO.
//!
//! Sem os defeitos do macOS: o spinner não gira para sempre (o `claude` que
//! diz "Login successful" e cuja conta não aparece em 12×400 ms vira um estado
//! com nome, "conferir de novo"); a casa reservada de um login que não virou
//! conta sai do disco ao fechar (podia ter recebido uma credencial de verdade);
//! e o relogin que volta com OUTRA conta não deixa o login dela na casa desta.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use router_core::engine::config_dir::ConfigDir;
use router_core::engine::router_config_store::{LoginOutcome, ReloginOutcome};
use router_core::usage::claude_binary::ClaudeBinary;
use router_core::Id;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::publish;
use crate::login_output::LoginEvent;
use crate::login_session::{LoginRequest, LoginSession, SessionEvent};
use crate::snapshot::Snapshot;
use crate::state::AppState;

/// A conferência no disco depois do "Login successful" (≙ macOS: 12 × 400 ms).
/// A credencial pode levar um instante para assentar.
const CONFIRM_ATTEMPTS: u32 = 12;
const CONFIRM_INTERVAL: Duration = Duration::from_millis(400);

/// Quanto esperar o `claude` cancelado sair antes de mexer na casa dele.
const CANCEL_WAIT: Duration = Duration::from_secs(5);

/// O evento que leva a fase do login à janela (`null` = fechado).
pub const LOGIN_CHANGED: &str = "login-changed";

/// Adicionar uma conta nova (numa casa reservada) ou relogar uma existente
/// (na casa dela, com o e-mail no `--email`).
#[derive(Clone, PartialEq, Eq, Debug)]
// A máquina de estados (fases, `advance`, limpeza, repetição) é lógica pura e
// mora em `flow.rs`; daqui para baixo é o que a move no app: a sessão ConPTY,
// a conferência no disco e os comandos que a janela chama.
mod flow;
pub use flow::*;

// MARK: - O fluxo no app

/// O que a janela recebe.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginView {
    /// Cresce a cada mudança, entre fluxos também. A resposta de um comando
    /// pode chegar DEPOIS do evento de uma mudança posterior (o link sai em
    /// milissegundos); a tela fica com a visão de revisão maior.
    pub revision: u64,
    pub relogin: bool,
    #[serde(flatten)]
    pub phase: LoginPhase,
    pub invalid_code: bool,
}

struct LoginFlow {
    /// Cada (re)começo tem o seu número: fato atrasado de uma sessão velha
    /// (cancelada, substituída) não mexe no fluxo novo.
    generation: u64,
    revision: u64,
    kind: LoginKind,
    home: ConfigDir,
    account_id: Id,
    session: Option<Arc<LoginSession>>,
    state: FlowState,
}

impl LoginFlow {
    fn view(&self) -> LoginView {
        LoginView {
            revision: self.revision,
            relogin: matches!(self.kind, LoginKind::Relogin { .. }),
            phase: self.state.phase.clone(),
            invalid_code: self.state.invalid_code,
        }
    }
}

/// O login da janela — um por vez.
#[derive(Default)]
pub struct LoginState {
    flow: Mutex<Option<LoginFlow>>,
    generation: AtomicU64,
    revision: AtomicU64,
}

impl LoginState {
    fn flow(&self) -> MutexGuard<'_, Option<LoginFlow>> {
        self.flow.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Marca uma mudança no fluxo (a visão dele passa a valer mais).
    fn touch(&self, flow: &mut LoginFlow) -> LoginView {
        flow.revision = self.revision.fetch_add(1, Ordering::SeqCst) + 1;
        flow.view()
    }

    /// Um login em andamento: a volta de rotação espera. Ela espelha grupo →
    /// casa da conta ativa e, num relogin dessa conta, pisaria na credencial
    /// nova que o `claude auth login` acabou de gravar, antes de ela chegar ao
    /// grupo (o macOS tinha essa corrida).
    pub fn in_progress(&self) -> bool {
        self.flow()
            .as_ref()
            .is_some_and(|f| f.state.phase.in_progress())
    }
}

fn emit(app: &AppHandle, view: Option<&LoginView>) {
    let _ = app.emit(LOGIN_CHANGED, view);
}

fn id(text: &str) -> Result<Id, String> {
    Id::parse(text).map_err(|e| format!("id inválido {text:?}: {e}"))
}

/// Sobe o `claude auth login` para um fluxo. O fluxo entra ANTES da sessão:
/// o link pode chegar antes de `LoginSession::start` voltar.
fn begin(app: &AppHandle, kind: LoginKind, home: ConfigDir, account_id: Id) -> LoginView {
    let state = app.state::<LoginState>();
    let generation = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let email = match &kind {
        LoginKind::Relogin { email, .. } => Some(email.clone()),
        LoginKind::Add { .. } => None,
    };
    let claude = ClaudeBinary::locate();
    let phase = if claude.is_some() {
        LoginPhase::Starting
    } else {
        LoginPhase::Failed {
            reason: FailReason::NoClaude,
        }
    };
    let mut view = {
        let mut guard = state.flow();
        let flow = guard.insert(LoginFlow {
            generation,
            revision: 0,
            kind,
            home: home.clone(),
            account_id,
            session: None,
            state: FlowState {
                phase,
                invalid_code: false,
            },
        });
        state.touch(flow)
    };
    if let Some(claude) = claude {
        let request = LoginRequest {
            claude,
            env: LoginRequest::environment(std::env::vars_os(), &home),
            home,
            email,
        };
        let handle = app.clone();
        let started =
            LoginSession::start(request, move |event| on_event(&handle, generation, event));
        let mut guard = state.flow();
        match (
            started,
            guard.as_mut().filter(|f| f.generation == generation),
        ) {
            (Ok(session), Some(flow)) => {
                flow.session = Some(session);
                view = flow.view();
            }
            // Fechado enquanto subia.
            (Ok(session), None) => session.cancel(),
            (Err(error), Some(flow)) => {
                flow.state.phase = LoginPhase::Failed {
                    reason: FailReason::Pty {
                        detail: error.to_string(),
                    },
                };
                view = state.touch(flow);
            }
            (Err(_), None) => {}
        }
    }
    emit(app, Some(&view));
    view
}

fn current_view(app: &AppHandle) -> Option<LoginView> {
    app.state::<LoginState>()
        .flow()
        .as_ref()
        .map(LoginFlow::view)
}

/// Um fato da sessão (nas threads dela).
fn on_event(app: &AppHandle, generation: u64, event: SessionEvent) {
    let state = app.state::<LoginState>();
    let (view, step) = {
        let mut guard = state.flow();
        let Some(flow) = guard.as_mut().filter(|f| f.generation == generation) else {
            return;
        };
        if matches!(event, SessionEvent::Exited(_)) {
            flow.session = None;
        }
        let step = advance(&mut flow.state, &event);
        let view = if step == Step::Nothing {
            flow.view()
        } else {
            state.touch(flow)
        };
        (view, step)
    };
    match step {
        Step::Nothing => {}
        Step::Changed => emit(app, Some(&view)),
        Step::Confirm { announced } => {
            emit(app, Some(&view));
            confirm(app.clone(), generation, announced);
        }
    }
}

/// Confere o desfecho no disco (identidade + credencial na casa), numa thread.
fn confirm(app: AppHandle, generation: u64, announced: bool) {
    let _ = thread::Builder::new()
        .name("login-confere".into())
        .spawn(move || {
            for attempt in 0..CONFIRM_ATTEMPTS {
                if attempt > 0 {
                    thread::sleep(CONFIRM_INTERVAL);
                }
                let target = app
                    .state::<LoginState>()
                    .flow()
                    .as_ref()
                    .filter(|f| f.generation == generation)
                    .map(|f| (f.kind.clone(), f.home.clone(), f.account_id));
                let Some((kind, home, account_id)) = target else {
                    return; // fechado ou substituído
                };
                if let Some(phase) = outcome(&app, &kind, &home, account_id) {
                    settle(&app, generation, phase);
                    return;
                }
            }
            settle(
                &app,
                generation,
                if announced {
                    LoginPhase::Timeout
                } else {
                    LoginPhase::Failed {
                        reason: FailReason::Ended,
                    }
                },
            );
        });
}

fn outcome(
    app: &AppHandle,
    kind: &LoginKind,
    home: &ConfigDir,
    account_id: Id,
) -> Option<LoginPhase> {
    let state = app.state::<AppState>();
    let mut store = state.store();
    match kind {
        LoginKind::Add { group } => match store.finish_pending_login(home, account_id, *group) {
            LoginOutcome::Pending => None,
            LoginOutcome::Added(account) => Some(LoginPhase::Added {
                label: account.label().to_string(),
            }),
            LoginOutcome::Duplicate { email } => Some(LoginPhase::Duplicate { email }),
        },
        LoginKind::Relogin { account, .. } => match store.finish_relogin(*account) {
            ReloginOutcome::Pending => None,
            ReloginOutcome::Renewed(account) => Some(LoginPhase::Renewed {
                label: account.label().to_string(),
            }),
            ReloginOutcome::WrongAccount { expected, got } => {
                Some(LoginPhase::WrongAccount { expected, got })
            }
        },
    }
}

/// Grava o desfecho e republica o quadro (a conta nova aparece no cartão e na
/// bandeja).
fn settle(app: &AppHandle, generation: u64, phase: LoginPhase) {
    let view = {
        let state = app.state::<LoginState>();
        let mut guard = state.flow();
        let Some(flow) = guard.as_mut().filter(|f| f.generation == generation) else {
            return;
        };
        flow.state.phase = phase;
        state.touch(flow)
    };
    publish(app);
    emit(app, Some(&view));
}

/// A limpeza que fechar (ou recomeçar) faz no disco.
fn clean_up(app: &AppHandle, flow: &LoginFlow) {
    let state = app.state::<AppState>();
    let store = state.store();
    match cleanup_on_close(&flow.kind, &flow.state.phase) {
        Cleanup::Nothing => {}
        Cleanup::DiscardPendingHome => {
            store.discard_pending_home(&flow.home, flow.account_id);
        }
        Cleanup::DiscardWrongRelogin => {
            if let LoginKind::Relogin { account, .. } = &flow.kind {
                store.discard_wrong_relogin(*account);
            }
        }
    }
}

/// Tira o fluxo, encerra o `claude` (esperando ele sair) e limpa o disco.
fn take_and_close(app: &AppHandle) -> Option<LoginFlow> {
    let flow = app.state::<LoginState>().flow().take()?;
    if let Some(session) = &flow.session {
        session.cancel_and_wait(CANCEL_WAIT);
    }
    clean_up(app, &flow);
    Some(flow)
}

/// Fecha o login sem a janela pedir (a janela foi destruída).
pub fn close_quietly(app: &AppHandle) {
    if take_and_close(app).is_some() {
        emit(app, None);
        publish(app);
    }
}

/// "Adicionar conta": o login oficial numa casa reservada nova.
#[tauri::command]
pub async fn start_login(app: AppHandle, group_id: String) -> Result<LoginView, String> {
    let group = id(&group_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        take_and_close(&app);
        let (home, account_id) = app.state::<AppState>().store().new_account_home();
        begin(&app, LoginKind::Add { group }, home, account_id)
    })
    .await
    .map_err(|e| e.to_string())
}

/// "Relogar…": na casa da própria conta, com o e-mail dela no `--email`.
#[tauri::command]
pub async fn start_relogin(
    app: AppHandle,
    account_id: String,
    group_id: String,
) -> Result<LoginView, String> {
    let (account, group) = (id(&account_id)?, id(&group_id)?);
    tauri::async_runtime::spawn_blocking(move || {
        take_and_close(&app);
        let found = app
            .state::<AppState>()
            .store()
            .config()
            .account(account)
            .cloned();
        let Some(found) = found else {
            return Err(format!("conta desconhecida {account_id}"));
        };
        Ok(begin(
            &app,
            LoginKind::Relogin {
                account,
                group,
                email: found.identity.email.clone(),
            },
            found.home.clone(),
            account,
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// O login aberto agora (a janela reaberta volta a mostrá-lo).
#[tauri::command]
pub fn current_login(app: AppHandle) -> Option<LoginView> {
    current_view(&app)
}

/// Cola o código que a página mostrou. Apaga o aviso de código inválido.
#[tauri::command]
pub fn login_submit_code(app: AppHandle, code: String) -> bool {
    let (session, view) = {
        let state = app.state::<LoginState>();
        let mut guard = state.flow();
        let Some(flow) = guard.as_mut() else {
            return false;
        };
        flow.state.invalid_code = false;
        (flow.session.clone(), state.touch(flow))
    };
    emit(&app, Some(&view));
    session.is_some_and(|s| s.submit_code(&code))
}

/// "Tentar de novo": conta nova numa casa NOVA (a velha sai); relogin na casa
/// dele, sem o login estranho que ficou lá.
#[tauri::command]
pub async fn login_retry(app: AppHandle) -> Result<Option<LoginView>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let plan = {
            let state = app.state::<LoginState>();
            let guard = state.flow();
            let flow = guard.as_ref()?;
            retry_plan(&flow.kind, &flow.state.phase)
        };
        let Some(plan) = plan else {
            return current_view(&app);
        };
        let flow = take_and_close(&app)?;
        let (home, account_id) = match plan {
            Retry::NewHome => app.state::<AppState>().store().new_account_home(),
            Retry::SameHome => (flow.home, flow.account_id),
        };
        Some(begin(&app, flow.kind, home, account_id))
    })
    .await
    .map_err(|e| e.to_string())
}

/// "Conferir de novo", depois do tempo esgotado.
#[tauri::command]
pub fn login_recheck(app: AppHandle) -> Option<LoginView> {
    let (generation, view) = {
        let state = app.state::<LoginState>();
        let mut guard = state.flow();
        let flow = guard.as_mut()?;
        if flow.state.phase != LoginPhase::Timeout {
            return Some(flow.view());
        }
        flow.state.phase = LoginPhase::Confirming;
        (flow.generation, state.touch(flow))
    };
    emit(&app, Some(&view));
    confirm(app, generation, true);
    Some(view)
}

/// Fecha a tela do login: encerra o `claude` se ainda roda e faz a limpeza
/// (a casa reservada que não virou conta; o login estranho de um relogin).
#[tauri::command]
pub async fn login_close(app: AppHandle) -> Result<Snapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        take_and_close(&app);
        emit(&app, None);
        publish(&app)
    })
    .await
    .map_err(|e| e.to_string())
}
