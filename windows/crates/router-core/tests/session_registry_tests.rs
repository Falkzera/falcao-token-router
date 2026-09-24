//! `SessionRegistry`: quais sessões do Claude Code estão vivas em cada perfil,
//! lidas do `<perfil>\sessions\<pid>.json` que ele próprio escreve.
//!
//! Portados de `SessionRegistryTests.swift` (8), com o registro REAL do Windows
//! (anonimizado) no lugar do de macOS, e as regressões do Windows: `procStart`
//! é um FILETIME em texto (100 ns desde 1601, UTC — bateu exatamente com o
//! início do processo vivo no spike de 22/09/2026); liveness por `OpenProcess` +
//! `GetProcessTimes`; registro de outra máquina (`pidDomain`) não é sessão daqui.

use chrono::{TimeZone, Utc};
use serde_json::{json, Value};

use router_core::engine::session_registry::{
    LiveSession, ProcessLiveness, SessionRegistry, SessionStatus,
};
use router_core::platform::host::local_host_names;
use router_core::platform::process_times::{self, to_filetime};
use router_core::ConfigDir;

/// Um registro REAL do Windows (Claude Code 2.1.280), anonimizado. O `pid` e o
/// `procStart` são trocados por teste; o resto é como o Claude Code escreve.
fn record(pid: u32, status: &str, proc_start: &str) -> Value {
    json!({
        "pid": pid,
        "sessionId": "00000000-0000-4000-8000-000000000000",
        "cwd": "C:\\Users\\exemplo\\Projects\\falcao-token-router",
        "startedAt": 1_790_104_714_946_i64,
        "procStart": proc_start,
        "version": "2.1.280",
        "peerProtocol": 1,
        "peerFeatures": ["notify_idle", "artifact_yield"],
        "kind": "interactive",
        "entrypoint": "cli",
        "pidDomain": "win32:exemplo-pc",
        "messagingSocketPath": "\\\\.\\pipe\\LOCAL\\cc-msg-0000",
        "name": "falcao-token-router-9e",
        "nameSource": "derived",
        "nameSince": 1_790_104_714_947_i64,
        "status": status,
        "updatedAt": 1_790_105_178_990_i64,
        "statusUpdatedAt": 1_790_105_178_990_i64
    })
}

/// Um pid que não existe: os pids do Windows são múltiplos de 4 e nunca chegam
/// perto disto.
const DEAD_PID: u32 = 0x7FFF_FFFC;

fn own_pid_record(start: &str) -> Value {
    let mut r = record(std::process::id(), "busy", start);
    // Registro desta máquina (o domínio do fixture é de outra).
    r["pidDomain"] = json!(format!("win32:{}", local_host_names()[0].to_lowercase()));
    r
}

fn own_start_filetime() -> String {
    let start = process_times::start_time(std::process::id()).expect("início do teste");
    to_filetime(start).to_string()
}

/// Roda o registro com um só arquivo, com o leitor e a listagem em memória.
fn live_from(value: Value) -> Vec<LiveSession> {
    let bytes = serde_json::to_vec(&value).unwrap();
    SessionRegistry::live_sessions_with(
        &ConfigDir::dedicated("C:/Users/exemplo/perfil"),
        |dir| vec![dir.join("1234.json")],
        |_| Some(bytes.clone()),
    )
}

#[test]
fn reads_a_real_record() {
    let s = SessionRegistry::session_from(&record(4242, "busy", "134345783118617709")).unwrap();
    assert_eq!(s.pid, 4242);
    assert_eq!(s.cwd, "C:\\Users\\exemplo\\Projects\\falcao-token-router");
    assert_eq!(s.folder(), "falcao-token-router");
    assert_eq!(s.label(), "falcao-token-router-9e");
    assert_eq!(s.status, SessionStatus::Busy);
    assert_eq!(
        s.session_id.as_deref(),
        Some("00000000-0000-4000-8000-000000000000")
    );
}

/// Um estado que este app não conhece ainda é um estado. Virar `idle` seria
/// afirmar que a sessão está parada — a pior leitura quando a pergunta é se dá
/// para trocar a conta agora.
#[test]
fn an_unknown_status_is_other_not_idle() {
    let s = SessionRegistry::session_from(&record(1, "compacting", "134345783118617709")).unwrap();
    assert_eq!(s.status, SessionStatus::Other("compacting".into()));
    assert!(!s.status.is_engaged());
    assert_eq!(SessionStatus::from_raw(Some("shell")), SessionStatus::Shell);
    assert!(SessionStatus::from_raw(Some("waiting")).is_engaged());
    assert!(SessionStatus::from_raw(Some("busy")).is_engaged());
    assert!(!SessionStatus::from_raw(Some("idle")).is_engaged());
}

/// Sessão que morreu de forma abrupta deixa o arquivo dizendo `busy` para
/// sempre. Confiar no arquivo mostraria trabalho que não existe.
#[test]
fn a_dead_process_record_is_dropped() {
    let mut dead = record(DEAD_PID, "busy", "134345783118617709");
    dead["pidDomain"] = json!(format!("win32:{}", local_host_names()[0].to_lowercase()));
    assert!(live_from(dead).is_empty());
}

/// O processo deste teste existe e o carimbo bate: tem de passar.
#[test]
fn a_live_process_with_a_coherent_stamp_passes() {
    let live = live_from(own_pid_record(&own_start_filetime()));
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].pid, std::process::id());
}

/// Numa máquina que fica dias ligada, pids são reciclados. Sem comparar o
/// instante de início, um pid reaproveitado ressuscitaria uma sessão morta.
#[test]
fn a_recycled_pid_is_refused_by_its_start_time() {
    let me = std::process::id();
    let in_2020 = Utc.timestamp_opt(1_600_000_000, 0).single().unwrap();
    assert!(!ProcessLiveness::is_alive(me, Some(in_2020)));
    // Sem carimbo não dá para provar nem desprovar: confia no pid.
    assert!(ProcessLiveness::is_alive(me, None));
}

/// `ctime` (o formato do macOS) preenche o dia com ESPAÇO quando tem um dígito
/// — e o dia da semana do fixture original está errado (08/09/2026 é terça), o
/// que o leitor tolera por não depender dele.
#[test]
fn a_ctime_proc_start_with_a_single_digit_day_is_read() {
    let with_double_space = SessionRegistry::parse_proc_start("Mon Sep  8 09:05:01 2026").unwrap();
    let with_single_space = SessionRegistry::parse_proc_start("Mon Sep 8 09:05:01 2026").unwrap();
    assert_eq!(with_double_space, with_single_space);
    assert_eq!(
        with_double_space,
        Utc.with_ymd_and_hms(2026, 9, 8, 9, 5, 1).unwrap()
    );
}

/// O registro é POR PERFIL, e fica DENTRO do diretório nos dois casos —
/// diferente do `.claude.json`, que no padrão mora ao lado.
#[test]
fn the_sessions_folder_is_inside_the_profile_in_both_cases() {
    assert_eq!(
        SessionRegistry::directory(&ConfigDir::standard("C:/Users/exemplo")),
        std::path::Path::new("C:/Users/exemplo/.claude").join("sessions")
    );
    assert_eq!(
        SessionRegistry::directory(&ConfigDir::dedicated("C:/grupos/g")),
        std::path::Path::new("C:/grupos/g").join("sessions")
    );
}

#[test]
fn a_record_without_pid_or_cwd_is_ignored_instead_of_breaking() {
    assert!(SessionRegistry::session_from(&json!({"cwd": "C:\\x"})).is_none());
    assert!(SessionRegistry::session_from(&json!({"pid": 1})).is_none());
    assert!(SessionRegistry::session_from(&json!("não é objeto")).is_none());
}

// --- Regressões do Windows ---

/// `procStart` no Windows é um FILETIME em texto: 100 ns desde 1601-01-01 UTC.
#[test]
fn a_filetime_proc_start_is_read_to_the_100ns() {
    let parsed = SessionRegistry::parse_proc_start("134345783118617709").unwrap();
    assert_eq!(
        parsed,
        Utc.timestamp_opt(1_790_104_711, 861_770_900)
            .single()
            .unwrap()
    );
    assert_eq!(to_filetime(parsed), 134_345_783_118_617_709);
}

/// O início do processo lido pelo `GetProcessTimes` é o deste teste: recente.
#[test]
fn the_start_time_of_this_process_is_recent() {
    let start = process_times::start_time(std::process::id()).unwrap();
    let age = Utc::now() - start;
    assert!(
        age >= chrono::Duration::zero() && age < chrono::Duration::minutes(30),
        "{age}"
    );
}

/// O `pidDomain` diz de que máquina é o pid. Um perfil sincronizado entre
/// máquinas (ou copiado de um Mac) traz registros cujos pids não significam
/// nada aqui — nem que coincidam com um processo vivo desta máquina.
#[test]
fn a_record_from_another_machine_is_not_a_session_here() {
    let start = own_start_filetime();
    let mut other_host = own_pid_record(&start);
    other_host["pidDomain"] = json!("win32:outra-maquina");
    assert!(live_from(other_host).is_empty());

    let mut other_os = own_pid_record(&start);
    other_os["pidDomain"] = json!("darwin:exemplo-mac");
    assert!(live_from(other_os).is_empty());

    // Sem `pidDomain` (registro antigo): confia no pid.
    let mut unknown = own_pid_record(&start);
    unknown.as_object_mut().unwrap().remove("pidDomain");
    assert_eq!(live_from(unknown).len(), 1);

    // A caixa do nome da máquina não importa.
    let mut upper = own_pid_record(&start);
    upper["pidDomain"] = json!(format!("win32:{}", local_host_names()[0].to_uppercase()));
    assert_eq!(live_from(upper).len(), 1);
}

/// Mais nova primeiro.
#[test]
fn live_sessions_come_newest_first() {
    let now_ft = own_start_filetime();
    let mut newer = own_pid_record(&now_ft);
    newer["name"] = json!("nova");
    // A "velha" não tem início nenhum (nem `procStart` nem `startedAt`): o pid
    // vivo basta para ela passar, e sem início ela vai para o fim da lista.
    let mut older = own_pid_record(&now_ft);
    older["name"] = json!("velha");
    older.as_object_mut().unwrap().remove("procStart");
    older.as_object_mut().unwrap().remove("startedAt");
    let files = [
        serde_json::to_vec(&older).unwrap(),
        serde_json::to_vec(&newer).unwrap(),
    ];

    let live = SessionRegistry::live_sessions_with(
        &ConfigDir::dedicated("C:/Users/exemplo/perfil"),
        |dir| vec![dir.join("1.json"), dir.join("2.json")],
        |path| {
            let i = usize::from(path.ends_with("2.json"));
            Some(files[i].clone())
        },
    );

    let names: Vec<&str> = live.iter().map(|s| s.label()).collect();
    assert_eq!(names, ["nova", "velha"]);
}
