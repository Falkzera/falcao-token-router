//! A trava do motor entre o app e a CLI (mutex nomeado do Windows) — novo no
//! porte. O macOS não tem trava; aqui o laço do app e um `router launch` no
//! terminal não escrevem a mesma credencial ao mesmo tempo.

use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use router_core::engine::engine_lock::EngineLock;

#[test]
fn a_second_holder_waits_for_the_first_to_release() {
    let base = tempfile::tempdir().unwrap();
    let path = base.path().to_path_buf();
    let (held_tx, held_rx) = mpsc::channel();
    let holder = thread::spawn(move || {
        let _guard = EngineLock::acquire(&path, Duration::from_secs(1)).expect("o 1º pega");
        held_tx.send(()).unwrap();
        thread::sleep(Duration::from_millis(300));
    });
    held_rx.recv().unwrap();

    assert!(
        EngineLock::acquire(base.path(), Duration::from_millis(30)).is_none(),
        "pegou junto com o dono"
    );
    let start = Instant::now();
    assert!(EngineLock::acquire(base.path(), Duration::from_secs(5)).is_some());
    assert!(start.elapsed() >= Duration::from_millis(100), "não esperou");
    holder.join().unwrap();
}

/// Um processo que morre com a trava (o app fechado à força no meio de uma
/// troca) não pode deixá-la presa para sempre.
#[test]
fn a_lock_left_by_a_dead_owner_is_taken_over() {
    let base = tempfile::tempdir().unwrap();
    let path = base.path().to_path_buf();
    thread::spawn(move || {
        let guard = EngineLock::acquire(&path, Duration::from_secs(1)).unwrap();
        std::mem::forget(guard); // morre sem soltar
    })
    .join()
    .unwrap();

    assert!(EngineLock::acquire(base.path(), Duration::from_secs(1)).is_some());
}

#[test]
fn different_bases_do_not_contend() {
    let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let _held = EngineLock::acquire(a.path(), Duration::from_secs(1)).unwrap();
    let other = b.path().to_path_buf();
    let got =
        thread::spawn(move || EngineLock::acquire(&other, Duration::from_millis(50)).is_some())
            .join()
            .unwrap();
    assert!(got);
}

/// App e CLI chegam ao MESMO nome para a mesma base, escrita de qualquer jeito.
#[test]
fn the_name_ignores_case_slash_style_and_trailing_slash() {
    let one = EngineLock::name_for(Path::new(
        r"C:\Users\exemplo\AppData\Local\com.synqo.falcao-router",
    ));
    let two = EngineLock::name_for(Path::new(
        "c:/users/exemplo/appdata/local/com.synqo.falcao-router/",
    ));
    assert_eq!(one, two);
    assert!(
        one.starts_with(r"Local\com.synqo.falcao-router.engine."),
        "{one}"
    );
}
