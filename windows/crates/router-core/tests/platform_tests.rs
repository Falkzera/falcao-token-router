//! E/S de arquivo do Windows: escrita atômica com **mtime novo** e nova
//! tentativa quando outro processo segura o arquivo sem compartilhar.
//!
//! O mtime novo não é detalhe: o Claude Code só relê a credencial quando o
//! mtime do `.credentials.json` muda (visto no binário 2.1.280 e confirmado no
//! spike de 22/09/2026). A nova tentativa cobre antivírus, indexador e o próprio
//! Claude Code com o arquivo aberto no meio de uma escrita.

use std::fs::{self, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::thread;
use std::time::{Duration, SystemTime};

use router_core::platform::atomic_write::{read_retrying, write_atomic};

/// Um ano 2001 qualquer — longe o bastante de "agora".
fn long_ago() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000)
}

#[test]
fn write_atomic_leaves_a_fresh_mtime() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(".credentials.json");
    fs::write(&path, b"antigo").unwrap();
    // Carimba o arquivo no passado, como uma cópia que preservasse o mtime faria.
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(long_ago())
        .unwrap();

    write_atomic(&path, b"novo").unwrap();

    let mtime = fs::metadata(&path).unwrap().modified().unwrap();
    let age = SystemTime::now()
        .duration_since(mtime)
        .unwrap_or(Duration::ZERO);
    assert!(age < Duration::from_secs(60), "mtime não é novo: {mtime:?}");
    assert_eq!(fs::read(&path).unwrap(), b"novo");
}

/// Outro processo abre o destino SEM compartilhar (antivírus, indexador): o
/// `rename` falha por violação de compartilhamento até ele soltar. A escrita
/// espera e tenta de novo, em vez de desistir de primeira.
#[test]
fn write_atomic_retries_while_the_file_is_held() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(".credentials.json");
    fs::write(&path, b"antigo").unwrap();

    let held = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    let releaser = thread::spawn(move || {
        thread::sleep(Duration::from_millis(150));
        drop(held);
    });

    write_atomic(&path, b"novo").unwrap();
    releaser.join().unwrap();

    assert_eq!(fs::read(&path).unwrap(), b"novo");
}

/// Segurado o tempo todo: desiste com erro, deixa o original intacto e não
/// larga temporário para trás.
#[test]
fn write_atomic_gives_up_cleanly_when_the_file_stays_held() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(".credentials.json");
    fs::write(&path, b"antigo").unwrap();

    let held = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    let result = write_atomic(&path, b"novo");
    drop(held);

    assert!(result.is_err(), "devia desistir com o arquivo preso");
    assert_eq!(fs::read(&path).unwrap(), b"antigo");
    let leftovers: Vec<_> = fs::read_dir(tmp.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(leftovers.len(), 1, "sobrou temporário: {leftovers:?}");
}

#[test]
fn read_retrying_waits_for_an_exclusive_holder() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(".claude.json");
    fs::write(&path, b"{}").unwrap();

    let held = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    let releaser = thread::spawn(move || {
        thread::sleep(Duration::from_millis(150));
        drop(held);
    });

    let bytes = read_retrying(&path).unwrap();
    releaser.join().unwrap();
    assert_eq!(bytes, b"{}");
}

#[test]
fn read_retrying_reports_a_missing_file_at_once() {
    let tmp = tempfile::tempdir().unwrap();
    let err = read_retrying(&tmp.path().join("nao-existe.json")).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}
