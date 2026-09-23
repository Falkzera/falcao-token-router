//! O laço do app enquanto ele vive (≙ `startRotation` do macOS).
//!
//! A cada 30 s relê o quadro (amostras do sensor, conta ativa, sessões vivas) e
//! redesenha a bandeja; a cada 180 s também roda a volta de rotação — espelha a
//! ativa de cada grupo e troca se ela passou do limiar. Três minutos para a
//! rotação porque o `rate_limits` só muda quando há atividade: reconsultar mais
//! rápido não traz número novo, só trabalho. A releitura é barata (arquivos
//! pequenos) e deixa o número da bandeja acompanhar o sensor.

use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;
use crate::tray;

/// De quanto em quanto tempo a bandeja relê o quadro.
const REFRESH_EVERY: Duration = Duration::from_secs(30);
/// A rotação roda a cada tantas releituras (6 × 30 s = 180 s, como no macOS).
const ROTATE_EVERY_TICKS: u64 = 6;

/// O evento que as janelas ouvem para reler o quadro.
pub const SNAPSHOT_CHANGED: &str = "snapshot-changed";

/// Uma volta: relê (e, se `rotate`, roda a rotação), redesenha, avisa as janelas.
pub fn tick(app: &AppHandle, rotate: bool) {
    {
        let state = app.state::<AppState>();
        let mut store = state.store();
        store.refresh_usage();
        if rotate {
            store.rotate_all();
        }
    }
    tray::refresh(app);
    let _ = app.emit(SNAPSHOT_CHANGED, ());
}

pub fn start(app: AppHandle) {
    let spawned = thread::Builder::new()
        .name("rotacao".into())
        .spawn(move || {
            // Como no macOS, a primeira volta (com rotação) é já na subida.
            let mut count: u64 = 0;
            loop {
                tick(&app, count.is_multiple_of(ROTATE_EVERY_TICKS));
                count += 1;
                thread::sleep(REFRESH_EVERY);
            }
        });
    if let Err(e) = spawned {
        eprintln!("falcao: o laço de rotação não subiu: {e}");
    }
}
