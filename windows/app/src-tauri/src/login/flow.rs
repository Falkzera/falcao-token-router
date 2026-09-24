//! A máquina de estados do login: as fases, o que cada evento do `claude` faz
//! com elas, e o que fica para limpar ou repetir.
//!
//! Sem nada de Tauri de propósito — é lógica pura, e é por isso que os testes
//! abaixo rodam sem app, sem ConPTY e sem disco. O que move essa máquina no
//! mundo real está no `mod.rs` ao lado.

use super::*;

pub enum LoginKind {
    Add {
        group: Id,
    },
    Relogin {
        account: Id,
        group: Id,
        email: String,
    },
}

/// Por que o login não terminou.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "code")]
pub enum FailReason {
    /// Não há `claude` nesta máquina.
    NoClaude,
    /// O ConPTY não subiu.
    Pty { detail: String },
    /// O `claude` saiu sem concluir.
    Ended,
    /// O `claude` recusou (`Login failed: …`) — o motivo, como ele disse.
    Refused { detail: String },
}

/// O que a tela do login mostra.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "phase")]
pub enum LoginPhase {
    Starting,
    /// O link apareceu (o navegador já abriu nele).
    Waiting {
        url: String,
    },
    /// O `claude` terminou; conferindo no disco.
    Confirming,
    Added {
        label: String,
    },
    Renewed {
        label: String,
    },
    /// Voltou uma conta que já está no grupo (o navegador não trocou de conta).
    Duplicate {
        email: String,
    },
    /// Relogin que voltou com outro e-mail.
    WrongAccount {
        expected: String,
        got: String,
    },
    Failed {
        reason: FailReason,
    },
    /// O `claude` disse que deu certo, e a conta não apareceu no disco.
    Timeout,
}

impl LoginPhase {
    /// Ainda esperando o `claude` (antes do fim anunciado).
    pub fn is_live(&self) -> bool {
        matches!(self, LoginPhase::Starting | LoginPhase::Waiting { .. })
    }

    /// Um login em andamento: a volta de rotação espera (ver `rotation_loop`).
    pub fn in_progress(&self) -> bool {
        self.is_live() || *self == LoginPhase::Confirming
    }
}

/// O estado que a tela acompanha.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FlowState {
    pub phase: LoginPhase,
    /// O último código colado não tinha a forma `código#state`.
    pub invalid_code: bool,
}

/// O que fazer depois de um fato da sessão.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    Nothing,
    Changed,
    /// Conferir no disco; `announced` = o `claude` disse "Login successful"
    /// (sem a conta no disco depois disso, é `Timeout`; sem o anúncio, `Ended`).
    Confirm {
        announced: bool,
    },
}

/// Um fato da sessão aplicado ao estado (puro).
pub fn advance(state: &mut FlowState, event: &SessionEvent) -> Step {
    let live = state.phase.is_live();
    match event {
        SessionEvent::Output(LoginEvent::Url(url)) if state.phase == LoginPhase::Starting => {
            state.phase = LoginPhase::Waiting { url: url.clone() };
            Step::Changed
        }
        SessionEvent::Output(LoginEvent::InvalidCode) if live => {
            state.invalid_code = true;
            Step::Changed
        }
        SessionEvent::Output(LoginEvent::Success) if live => {
            state.phase = LoginPhase::Confirming;
            state.invalid_code = false;
            Step::Confirm { announced: true }
        }
        SessionEvent::Output(LoginEvent::Failed(detail)) if live => {
            state.phase = LoginPhase::Failed {
                reason: FailReason::Refused {
                    detail: detail.clone(),
                },
            };
            Step::Changed
        }
        // Saiu com 0 sem anunciar: quem diz se deu certo é o disco.
        SessionEvent::Exited(Some(0)) if live => {
            state.phase = LoginPhase::Confirming;
            Step::Confirm { announced: false }
        }
        SessionEvent::Exited(_) if live => {
            state.phase = LoginPhase::Failed {
                reason: FailReason::Ended,
            };
            Step::Changed
        }
        _ => Step::Nothing,
    }
}

/// O que fechar a tela faz com o disco.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cleanup {
    Nothing,
    /// A casa reservada nunca virou conta: sai (com a credencial que tiver).
    DiscardPendingHome,
    /// O relogin gravou OUTRA conta na casa desta: a credencial estranha sai.
    DiscardWrongRelogin,
}

pub fn cleanup_on_close(kind: &LoginKind, phase: &LoginPhase) -> Cleanup {
    match (kind, phase) {
        (LoginKind::Add { .. }, LoginPhase::Added { .. }) => Cleanup::Nothing,
        // Mesmo "conferindo": se a conta entrou nesse meio-tempo, o núcleo
        // recusa apagar a casa de uma conta registrada.
        (LoginKind::Add { .. }, _) => Cleanup::DiscardPendingHome,
        (LoginKind::Relogin { .. }, LoginPhase::WrongAccount { .. }) => {
            Cleanup::DiscardWrongRelogin
        }
        // A casa de uma conta que existe nunca sai.
        (LoginKind::Relogin { .. }, _) => Cleanup::Nothing,
    }
}

/// Como recomeçar ("Tentar de novo").
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Retry {
    /// Numa casa NOVA (a velha sai: duplicada ou pela metade).
    NewHome,
    /// Na mesma casa (relogin), depois da limpeza que o fechar faria.
    SameHome,
}

pub fn retry_plan(kind: &LoginKind, phase: &LoginPhase) -> Option<Retry> {
    let retryable = match phase {
        LoginPhase::Duplicate { .. } | LoginPhase::WrongAccount { .. } | LoginPhase::Timeout => {
            true
        }
        LoginPhase::Failed { reason } => *reason != FailReason::NoClaude,
        _ => false,
    };
    retryable.then_some(match kind {
        LoginKind::Add { .. } => Retry::NewHome,
        LoginKind::Relogin { .. } => Retry::SameHome,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(phase: LoginPhase) -> FlowState {
        FlowState {
            phase,
            invalid_code: false,
        }
    }

    fn waiting() -> LoginPhase {
        LoginPhase::Waiting {
            url: "https://claude.com/cai/oauth/authorize?x=1".into(),
        }
    }

    fn out(event: LoginEvent) -> SessionEvent {
        SessionEvent::Output(event)
    }

    #[test]
    fn the_link_moves_to_waiting_once() {
        let mut s = state(LoginPhase::Starting);
        let url = "https://claude.com/cai/oauth/authorize?x=1".to_string();
        assert_eq!(
            advance(&mut s, &out(LoginEvent::Url(url.clone()))),
            Step::Changed
        );
        assert_eq!(s.phase, LoginPhase::Waiting { url });
        let again = "https://claude.com/cai/oauth/authorize?y=2".to_string();
        assert_eq!(advance(&mut s, &out(LoginEvent::Url(again))), Step::Nothing);
        assert_eq!(s.phase, waiting());
    }

    /// O anúncio leva à conferência no disco; o fim do processo que vem logo
    /// depois não confere de novo.
    #[test]
    fn success_confirms_on_disk_once() {
        let mut s = state(waiting());
        assert_eq!(
            advance(&mut s, &out(LoginEvent::Success)),
            Step::Confirm { announced: true }
        );
        assert_eq!(s.phase, LoginPhase::Confirming);
        assert_eq!(
            advance(&mut s, &SessionEvent::Exited(Some(0))),
            Step::Nothing
        );
        assert_eq!(s.phase, LoginPhase::Confirming);
    }

    /// Uma versão que sai com 0 sem anunciar também é conferida no disco — e,
    /// sem nada lá, é "encerrado", não "tempo esgotado".
    #[test]
    fn a_quiet_exit_zero_is_checked_on_disk() {
        let mut s = state(waiting());
        assert_eq!(
            advance(&mut s, &SessionEvent::Exited(Some(0))),
            Step::Confirm { announced: false }
        );
        assert_eq!(s.phase, LoginPhase::Confirming);
    }

    #[test]
    fn a_refusal_or_an_early_exit_is_a_failure() {
        let mut s = state(waiting());
        assert_eq!(
            advance(&mut s, &out(LoginEvent::Failed("status 403".into()))),
            Step::Changed
        );
        assert_eq!(
            s.phase,
            LoginPhase::Failed {
                reason: FailReason::Refused {
                    detail: "status 403".into()
                }
            }
        );
        // O fim do processo depois da recusa não troca o motivo.
        assert_eq!(
            advance(&mut s, &SessionEvent::Exited(Some(1))),
            Step::Nothing
        );

        let mut s = state(LoginPhase::Starting);
        assert_eq!(
            advance(&mut s, &SessionEvent::Exited(Some(1))),
            Step::Changed
        );
        assert_eq!(
            s.phase,
            LoginPhase::Failed {
                reason: FailReason::Ended
            }
        );
    }

    /// Código inválido: um aviso na tela (que o próximo código apaga, no
    /// comando), e o login continua esperando.
    #[test]
    fn an_invalid_code_is_flagged_and_the_wait_goes_on() {
        let mut s = state(waiting());
        assert_eq!(
            advance(&mut s, &out(LoginEvent::InvalidCode)),
            Step::Changed
        );
        assert!(s.invalid_code);
        assert_eq!(s.phase, waiting());
    }

    /// Depois de um desfecho, a sessão não muda mais nada.
    #[test]
    fn an_outcome_is_final() {
        for phase in [
            LoginPhase::Added {
                label: "conta1".into(),
            },
            LoginPhase::Timeout,
            LoginPhase::Failed {
                reason: FailReason::Ended,
            },
        ] {
            let mut s = state(phase.clone());
            assert_eq!(advance(&mut s, &out(LoginEvent::Success)), Step::Nothing);
            assert_eq!(
                advance(&mut s, &SessionEvent::Exited(Some(0))),
                Step::Nothing
            );
            assert_eq!(s.phase, phase);
        }
    }

    fn add() -> LoginKind {
        LoginKind::Add { group: Id::new() }
    }

    fn relogin() -> LoginKind {
        LoginKind::Relogin {
            account: Id::new(),
            group: Id::new(),
            email: "conta1@exemplo.com".into(),
        }
    }

    /// Fechar um login de conta NOVA que não virou conta apaga a casa
    /// reservada; o de uma conta que existe NUNCA apaga a casa dela — só tira
    /// a credencial estranha de um relogin que voltou com outra conta.
    #[test]
    fn closing_cleans_only_what_never_became_an_account() {
        let added = LoginPhase::Added {
            label: "conta1".into(),
        };
        let duplicate = LoginPhase::Duplicate {
            email: "conta1@exemplo.com".into(),
        };
        let wrong = LoginPhase::WrongAccount {
            expected: "conta1@exemplo.com".into(),
            got: "intrusa@exemplo.com".into(),
        };
        assert_eq!(cleanup_on_close(&add(), &added), Cleanup::Nothing);
        for phase in [
            LoginPhase::Starting,
            waiting(),
            LoginPhase::Confirming,
            duplicate.clone(),
            LoginPhase::Timeout,
            LoginPhase::Failed {
                reason: FailReason::Ended,
            },
        ] {
            assert_eq!(
                cleanup_on_close(&add(), &phase),
                Cleanup::DiscardPendingHome,
                "{phase:?}"
            );
        }
        assert_eq!(
            cleanup_on_close(&relogin(), &wrong),
            Cleanup::DiscardWrongRelogin
        );
        for phase in [
            LoginPhase::Starting,
            waiting(),
            LoginPhase::Renewed {
                label: "conta1".into(),
            },
            LoginPhase::Timeout,
        ] {
            assert_eq!(
                cleanup_on_close(&relogin(), &phase),
                Cleanup::Nothing,
                "{phase:?}"
            );
        }
    }

    /// "Tentar de novo": a conta nova recomeça numa casa NOVA (a duplicada sai
    /// — ≙ macOS); o relogin, na casa dele. Sem desfecho, não há o que tentar.
    #[test]
    fn retrying_uses_a_fresh_home_only_for_new_accounts() {
        let duplicate = LoginPhase::Duplicate {
            email: "conta1@exemplo.com".into(),
        };
        let wrong = LoginPhase::WrongAccount {
            expected: "conta1@exemplo.com".into(),
            got: "intrusa@exemplo.com".into(),
        };
        let failed = LoginPhase::Failed {
            reason: FailReason::Ended,
        };
        assert_eq!(retry_plan(&add(), &duplicate), Some(Retry::NewHome));
        assert_eq!(retry_plan(&add(), &failed), Some(Retry::NewHome));
        assert_eq!(
            retry_plan(&add(), &LoginPhase::Timeout),
            Some(Retry::NewHome)
        );
        assert_eq!(retry_plan(&relogin(), &wrong), Some(Retry::SameHome));
        assert_eq!(retry_plan(&relogin(), &failed), Some(Retry::SameHome));
        assert_eq!(retry_plan(&add(), &waiting()), None);
        assert_eq!(
            retry_plan(
                &add(),
                &LoginPhase::Added {
                    label: "conta1".into()
                }
            ),
            None
        );
        assert_eq!(
            retry_plan(
                &add(),
                &LoginPhase::Failed {
                    reason: FailReason::NoClaude
                }
            ),
            None,
            "sem claude, tentar de novo daria no mesmo"
        );
    }
}
