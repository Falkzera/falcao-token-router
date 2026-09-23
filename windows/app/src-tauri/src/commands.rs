//! Os comandos que a janela de Grupos chama. Cada ação muda o store, e depois
//! a bandeja é redesenhada, as janelas são avisadas (`snapshot-changed`) e o
//! quadro novo volta para quem chamou — o botão que mudou algo vê o efeito na
//! mesma resposta.

use std::thread;

use router_core::engine::router_config_store::RouterConfigStore;
use router_core::usage::claude_binary::ClaudeBinary;
use router_core::usage::claude_usage_probe::ClaudeUsageProbe;
use router_core::Id;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::rotation_loop::SNAPSHOT_CHANGED;
use crate::snapshot::{self, Snapshot};
use crate::state::AppState;
use crate::tray;

/// Um id que veio do front. Id inválido é defeito do front, não do usuário.
fn id(text: &str) -> Result<Id, String> {
    Id::parse(text).map_err(|e| format!("id inválido {text:?}: {e}"))
}

/// Republica o quadro: bandeja, janelas, e a resposta.
pub fn publish(app: &AppHandle) -> Snapshot {
    let state = app.state::<AppState>();
    let snapshot = snapshot::build(&state.store(), state.measuring());
    tray::refresh(app);
    let _ = app.emit(SNAPSHOT_CHANGED, ());
    snapshot
}

/// Muda o store (sob a trava dele) e republica — a trava sai antes da
/// bandeja, que a pega de novo.
fn mutate(app: &AppHandle, change: impl FnOnce(&mut RouterConfigStore)) -> Snapshot {
    {
        let state = app.state::<AppState>();
        let mut store = state.store();
        change(&mut store);
    }
    publish(app)
}

#[tauri::command]
pub fn add_group(app: AppHandle, name: String, as_default: bool) -> Snapshot {
    let name = name.trim().to_string();
    mutate(&app, |store| {
        if !name.is_empty() {
            store.add_group_with(&name, as_default);
        }
    })
}

#[tauri::command]
pub fn rename_group(app: AppHandle, group_id: String, name: String) -> Result<Snapshot, String> {
    let group = id(&group_id)?;
    let name = name.trim().to_string();
    Ok(mutate(&app, |store| {
        if !name.is_empty() {
            store.rename_group(group, &name);
        }
    }))
}

#[tauri::command]
pub fn set_auto_rotate(app: AppHandle, group_id: String, on: bool) -> Result<Snapshot, String> {
    let group = id(&group_id)?;
    Ok(mutate(&app, |store| store.set_auto_rotate(group, on)))
}

/// Grava o limiar — o front só chama ao SOLTAR o controle (o macOS gravava a
/// cada passo do arrasto).
#[tauri::command]
pub fn set_threshold(app: AppHandle, group_id: String, percent: f64) -> Result<Snapshot, String> {
    let group = id(&group_id)?;
    Ok(mutate(&app, |store| store.set_threshold(group, percent)))
}

#[tauri::command]
pub fn reorder_accounts(
    app: AppHandle,
    group_id: String,
    account_ids: Vec<String>,
) -> Result<Snapshot, String> {
    let group = id(&group_id)?;
    let ordered = account_ids
        .iter()
        .map(|a| id(a))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(mutate(&app, |store| {
        store.reorder_accounts(group, &ordered)
    }))
}

#[tauri::command]
pub fn remove_group(app: AppHandle, group_id: String) -> Result<Snapshot, String> {
    let group = id(&group_id)?;
    Ok(mutate(&app, |store| store.remove_group(group)))
}

#[tauri::command]
pub fn make_default(app: AppHandle, group_id: String) -> Result<Snapshot, String> {
    let group = id(&group_id)?;
    Ok(mutate(&app, |store| store.make_default(group)))
}

#[tauri::command]
pub fn clear_default(app: AppHandle) -> Snapshot {
    mutate(&app, RouterConfigStore::clear_default)
}

/// "Usar": troca manual da conta que serve o grupo. Falha vira o erro do
/// quadro (com o motivo como fato).
#[tauri::command]
pub fn activate_account(
    app: AppHandle,
    account_id: String,
    group_id: String,
) -> Result<Snapshot, String> {
    let (account, group) = (id(&account_id)?, id(&group_id)?);
    Ok(mutate(&app, |store| {
        let found = (
            store.config().account(account).cloned(),
            store
                .config()
                .groups
                .iter()
                .find(|g| g.id == group)
                .cloned(),
        );
        if let (Some(account), Some(group)) = found {
            store.activate(&account, &group);
        }
    }))
}

/// Remove a conta do grupo e APAGA o login dela (credencial e casa) — a
/// confirmação da tela diz isso.
#[tauri::command]
pub fn remove_account(app: AppHandle, account_id: String) -> Result<Snapshot, String> {
    let account = id(&account_id)?;
    Ok(mutate(&app, |store| store.remove_account(account)))
}

#[tauri::command]
pub fn dismiss_error(app: AppHandle) -> Snapshot {
    mutate(&app, RouterConfigStore::clear_last_error)
}

/// O login que o `~\.claude` tem e que o router não conhece — o que a tela
/// avisa antes de um grupo virar padrão.
#[tauri::command]
pub fn foreign_default_login(state: State<'_, AppState>) -> Option<String> {
    state.store().foreign_default_login()
}

/// "Medir contas": a sonda roda numa thread, fora da trava do store (segundos
/// por conta), e o quadro é republicado ao começar (spinner) e ao terminar.
/// Uma medição por vez.
#[tauri::command]
pub fn measure_group(app: AppHandle, group_id: String) -> Result<Snapshot, String> {
    let group = id(&group_id)?;
    let state = app.state::<AppState>();
    if state.measuring().is_some() {
        return Ok(publish(&app));
    }
    let plan = {
        let store = state.store();
        let Some(found) = store.config().groups.iter().find(|g| g.id == group) else {
            return Ok(snapshot::build(&store, state.measuring()));
        };
        store.measure_plan(found)
    };
    let Some(claude) = ClaudeBinary::locate() else {
        return Ok(mutate(&app, RouterConfigStore::measure_unavailable));
    };
    state.set_measuring(Some(group));
    let started = publish(&app);
    let worker = app.clone();
    thread::spawn(move || {
        let probe = ClaudeUsageProbe::system(claude, plan.base.clone());
        let summary = plan.run(&probe);
        let state = worker.state::<AppState>();
        state.store().finish_measure(summary);
        state.set_measuring(None);
        publish(&worker);
    });
    Ok(started)
}
