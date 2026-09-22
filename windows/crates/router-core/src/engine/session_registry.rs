//! Quais sessões do Claude Code estão vivas **em cada perfil**
//! (≙ `SessionRegistry.swift`).
//!
//! O Claude Code grava um `<perfil>\sessions\<pid>.json` por sessão, e o registro
//! é por perfil — o que diz **qual sessão roda em qual grupo**, e portanto por
//! qual conta ela é atendida. É o contra-veneno do modo de falha silencioso: num
//! terminal aberto antes da integração, `claude trabalho` vira argumento e a
//! sessão sobe no `~\.claude`, na conta errada; vendo as sessões por perfil, o
//! erro aparece. Nada aqui escreve.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde_json::Value;

use super::config_dir::ConfigDir;
use crate::platform::host::local_host_names;
use crate::platform::process_times::{self, ProcessState};

/// O que a sessão está fazendo. Observados: `busy`, `idle`, `shell`; `waiting`
/// quando ela pede permissão. Desconhecido vira `Other` com o nome cru — um
/// estado que este app não entende ainda é um estado, e fingir que é ocioso é a
/// pior das leituras possíveis.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SessionStatus {
    Busy,
    Idle,
    Shell,
    Waiting,
    Other(String),
}

impl SessionStatus {
    pub fn from_raw(raw: Option<&str>) -> Self {
        match raw {
            Some("busy") => SessionStatus::Busy,
            Some("idle") => SessionStatus::Idle,
            Some("shell") => SessionStatus::Shell,
            Some("waiting") => SessionStatus::Waiting,
            Some(other) => SessionStatus::Other(other.to_string()),
            None => SessionStatus::Other(String::new()),
        }
    }

    /// Trabalhando ou esperando o usuário — os dois casos em que trocar a conta
    /// por baixo da sessão se faz sentir.
    pub fn is_engaged(&self) -> bool {
        matches!(self, SessionStatus::Busy | SessionStatus::Waiting)
    }
}

/// Uma sessão do Claude Code, lida do registro que ele próprio escreve.
#[derive(Clone, PartialEq, Debug)]
pub struct LiveSession {
    pub pid: u32,
    pub session_id: Option<String>,
    /// Onde a sessão roda — o que dá nome útil a ela na tela.
    pub cwd: String,
    /// O nome que o Claude Code derivou (`falcao-token-router-9e`).
    pub name: Option<String>,
    /// O início do PROCESSO (`procStart`), ou o do registro (`startedAt`).
    pub started_at: Option<DateTime<Utc>>,
    pub status: SessionStatus,
    pub status_updated_at: Option<DateTime<Utc>>,
    /// De que máquina é o pid: `win32:<host>`.
    pub pid_domain: Option<String>,
}

impl LiveSession {
    /// A última pasta do `cwd` — o rótulo curto que cabe numa linha.
    pub fn folder(&self) -> &str {
        let trimmed = self.cwd.trim_end_matches(['\\', '/']);
        trimmed.rsplit(['\\', '/']).next().unwrap_or(trimmed)
    }

    /// O rótulo de tela: o nome derivado se houver, senão a pasta.
    pub fn label(&self) -> &str {
        match &self.name {
            Some(name) if !name.is_empty() => name,
            _ => self.folder(),
        }
    }
}

pub struct SessionRegistry;

impl SessionRegistry {
    /// Onde o Claude Code registra as sessões de um perfil: **dentro** dele nos
    /// dois casos (diferente do `.claude.json`, que no padrão mora ao lado).
    pub fn directory(dir: &ConfigDir) -> PathBuf {
        dir.path().join("sessions")
    }

    /// As sessões vivas de um perfil, do disco.
    pub fn live_sessions(dir: &ConfigDir) -> Vec<LiveSession> {
        Self::live_sessions_with(
            dir,
            |folder| {
                fs::read_dir(folder)
                    .map(|entries| entries.filter_map(Result::ok).map(|e| e.path()).collect())
                    .unwrap_or_default()
            },
            |path| fs::read(path).ok(),
        )
    }

    /// O mesmo, com listagem e leitura injetáveis (testes).
    ///
    /// Um arquivo sobrevive ao processo que o escreveu: sessão que morreu de
    /// forma abrupta deixa o registro dizendo `busy` para sempre. Por isso cada
    /// entrada é conferida contra o processo de verdade — e contra a máquina:
    /// registro de outro `pidDomain` não é sessão daqui.
    pub fn live_sessions_with(
        dir: &ConfigDir,
        listing: impl Fn(&Path) -> Vec<PathBuf>,
        read: impl Fn(&Path) -> Option<Vec<u8>>,
    ) -> Vec<LiveSession> {
        let hosts = local_host_names();
        let mut sessions: Vec<LiveSession> = listing(&Self::directory(dir))
            .into_iter()
            .filter(|p| {
                p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("json"))
            })
            .filter_map(|p| read(&p))
            .filter_map(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .filter_map(|value| Self::session_from(&value))
            .filter(|s| Self::is_local_domain(s.pid_domain.as_deref(), &hosts))
            .filter(|s| ProcessLiveness::is_alive(s.pid, s.started_at))
            .collect();
        // Mais nova primeiro; sem início vai para o fim.
        sessions.sort_by_key(|s| std::cmp::Reverse(s.started_at));
        sessions
    }

    /// Decodificado com tolerância de propósito: o arquivo é escrito por outro
    /// programa, no calendário de release dele, e um campo novo nunca pode custar
    /// uma sessão que daria para mostrar. Exige só `pid` e `cwd`.
    pub fn session_from(raw: &Value) -> Option<LiveSession> {
        let obj = raw.as_object()?;
        let pid = u32::try_from(obj.get("pid")?.as_u64()?).ok()?;
        let cwd = obj.get("cwd")?.as_str()?.to_string();
        let text = |key: &str| obj.get(key).and_then(Value::as_str).map(String::from);
        let millis = |key: &str| {
            obj.get(key)
                .and_then(Value::as_f64)
                .and_then(|ms| Utc.timestamp_millis_opt(ms as i64).single())
        };

        // `procStart` é o instante do PROCESSO; `startedAt`, o do registro, uns
        // segundos depois. Para conferir um pid reciclado vale o primeiro.
        let started_at = obj
            .get("procStart")
            .and_then(Value::as_str)
            .and_then(Self::parse_proc_start)
            .or_else(|| millis("startedAt"));

        Some(LiveSession {
            pid,
            session_id: text("sessionId"),
            cwd,
            name: text("name"),
            started_at,
            status: SessionStatus::from_raw(obj.get("status").and_then(Value::as_str)),
            status_updated_at: millis("statusUpdatedAt"),
            pid_domain: text("pidDomain"),
        })
    }

    /// `procStart` em dois formatos: no Windows, FILETIME em texto
    /// (`134345783118617709`); no macOS, `ctime` em UTC (`Fri Sep 18 12:14:31
    /// 2026`, com espaço duplo no dia de um dígito). O dia da semana do `ctime`
    /// é ignorado — o leitor não depende dele estar certo.
    pub fn parse_proc_start(text: &str) -> Option<DateTime<Utc>> {
        let text = text.trim();
        if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) {
            return process_times::from_filetime(text.parse().ok()?);
        }
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() != 5 {
            return None;
        }
        NaiveDateTime::parse_from_str(&parts[1..].join(" "), "%b %d %H:%M:%S %Y")
            .ok()
            .map(|naive| naive.and_utc())
    }

    /// O pid é desta máquina? Sem `pidDomain` (registro antigo) ou num formato
    /// que não se entende: confia. `win32:<host>` com um dos nomes daqui: sim.
    /// Outra plataforma ou outro host: não.
    pub fn is_local_domain(domain: Option<&str>, hosts: &[String]) -> bool {
        let Some((platform, host)) = domain.and_then(|d| d.split_once(':')) else {
            return true;
        };
        platform.eq_ignore_ascii_case("win32") && hosts.iter().any(|h| h.eq_ignore_ascii_case(host))
    }
}

/// O pid ainda existe — e ainda é o **mesmo** processo?
///
/// Conferir o pid sozinho não basta numa máquina que fica dias ligada: pids são
/// reciclados, e um reaproveitado ressuscitaria uma sessão morta. Comparar o
/// instante de início resolve. Tolerância e o "sem prova, confia no pid" são os
/// do macOS, como o `PORTING.md` pede.
pub struct ProcessLiveness;

impl ProcessLiveness {
    /// Larga o bastante para a distância entre o processo subir e a sessão se
    /// registrar; apertada o bastante para um pid reciclado não passar.
    pub const REUSE_TOLERANCE_MS: i64 = 5 * 60 * 1000;

    pub fn is_alive(pid: u32, started_at: Option<DateTime<Utc>>) -> bool {
        match process_times::probe(pid) {
            ProcessState::Missing => false,
            // Existe, de outro dono: sem como provar nem desprovar, confia.
            ProcessState::Denied => true,
            ProcessState::Running(real) => match (started_at, real) {
                (Some(recorded), Some(real)) => {
                    (real - recorded).num_milliseconds().abs() < Self::REUSE_TOLERANCE_MS
                }
                _ => true,
            },
        }
    }
}
