//! A medição automática de 5 em 5 horas (21/10/2026, pedido do usuário para a
//! aba Histórico). O laço a chama junto com a rotação (a cada 180 s); QUEM
//! vence é regra do núcleo (`MeasureSchedule`) e o plano é do store
//! (`scheduled_measure_plans`). Aqui ficam as condições para rodar e a thread
//! que roda os planos fora da trava do store, como o "Medir contas".
//!
//! Silenciosa: falha da agenda não abre o aviso vermelho — ele é da última
//! ação DO USUÁRIO. A conta que não respondeu vira um buraco no gráfico, e a
//! idade da amostra dela continua à vista na aba Grupos.

use std::thread;

use chrono::Utc;
use router_core::usage::claude_binary::ClaudeBinary;
use router_core::usage::claude_usage_probe::ClaudeUsageProbe;
use tauri::{AppHandle, Manager};

use crate::commands::publish;
use crate::login::LoginState;
use crate::settings::SettingsStore;
use crate::state::AppState;

/// Olha a agenda e, se alguma conta venceu, mede numa thread. Não faz nada com
/// a chave desligada, com uma medição em curso (a do botão inclusive) ou com
/// um RELOGIN em andamento — a sonda poderia mexer na casa que está sendo
/// relogada (a rotação espera pelo mesmo motivo). "Adicionar conta" não segura:
/// a casa reservada ainda não é conta, e a sonda não a mede.
pub fn check(app: &AppHandle) {
    let on = app
        .try_state::<SettingsStore>()
        .is_some_and(|settings| settings.get().scheduled_measure);
    let signing_in = app
        .try_state::<LoginState>()
        .is_some_and(|login| login.relogin_in_progress());
    let state = app.state::<AppState>();
    if !on || signing_in || state.measuring().is_some() {
        return;
    }
    let now = Utc::now();
    let attempts = state.measure_attempts();
    let plans = state.store().scheduled_measure_plans(now, &attempts);
    let Some((first, plan)) = plans.first() else {
        return;
    };
    // Sem `claude` não há sonda — e nada a dizer: ninguém pediu.
    let Some(claude) = ClaudeBinary::locate() else {
        return;
    };
    let probe = ClaudeUsageProbe::system(claude, plan.base.clone());
    if !state.try_start_measuring(*first) {
        return;
    }
    state.record_attempts(
        plans
            .iter()
            .flat_map(|(_, plan)| plan.targets.iter().map(|t| t.account_id)),
        now,
    );
    let worker = app.clone();
    let spawned = thread::Builder::new()
        .name("medicao-agendada".into())
        .spawn(move || {
            let state = worker.state::<AppState>();
            for (group, plan) in &plans {
                // A marca passa de grupo em grupo sem se apagar no meio: o
                // botão não começa outra medição entre dois grupos.
                state.set_measuring(Some(*group));
                publish(&worker);
                let summary = plan.run(&probe);
                let mut store = state.store();
                store.note_login_state(&summary);
                store.refresh_usage();
            }
            state.set_measuring(None);
            publish(&worker);
        });
    if spawned.is_err() {
        state.set_measuring(None);
    }
}
