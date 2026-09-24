//! A sonda ativa: pergunta o uso ao **binário oficial**, não ao endpoint
//! (≙ `ClaudeUsageProbe.swift`).
//!
//! Existe pela lacuna que o sensor passivo não cobre: o `rate_limits` da status
//! line traz só as janelas de 5h e 7 dias, e o limite POR MODELO — o que estoura
//! primeiro — não vem ali. `claude --print /usage` imprime as três linhas. Quem
//! faz a requisição é o cliente oficial, com a credencial dele — o mesmo que o
//! usuário digitar `/usage`. O router continua sem tocar em endpoint nenhum.
//!
//! Custa um processo subindo do zero e uma requisição por conta: é sob demanda
//! (`router measure`, "Medir contas"), nunca o laço.
//!
//! O Windows imprime diferente do macOS (captura do spike de 22/09/2026): a data
//! vem com vírgula (`Sep 22, 8:40pm`, não `Sep 22 at 8:40pm`), as linhas
//! terminam em CRLF, e um perfil deslogado sai com código **0** e nenhuma linha
//! `Current` (só o resumo do `--print`) — "sem login" é decidido pela ausência
//! das linhas, não pelo código de saída.

use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{mpsc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{DateTime, Datelike, Local, LocalResult, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::claude_binary::ClaudeCommand;
use crate::engine::config_dir::ConfigDir;
use crate::engine::engine_lock::EngineLock;
use crate::engine::group_usage::{GroupUsageSample, GroupUsageStore, ModelUsage, UsageOrigin};
use crate::engine::provider_env::ProviderEnv;
use crate::ids::Id;

/// Uma janela por modelo, como o `/usage` a nomeia.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelWindow {
    /// O nome de exibição que o `/usage` imprime (`Fable`, `Opus`…). Cru, e não
    /// mapeado para um enum: a lista de modelos muda sem avisar, e um nome que
    /// este app não conhece ainda é um limite que estoura.
    pub name: String,
    /// 0–1.
    pub percent: f64,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::time_fmt::optional"
    )]
    pub resets_at: Option<DateTime<Utc>>,
}

impl ModelWindow {
    pub fn new(name: impl Into<String>, percent: f64, resets_at: Option<DateTime<Utc>>) -> Self {
        ModelWindow {
            name: name.into(),
            percent,
            resets_at,
        }
    }
}

/// O que a sonda leu de uma conta.
#[derive(Clone, PartialEq, Debug)]
pub struct Reading {
    pub session: Option<f64>,
    pub session_resets_at: Option<DateTime<Utc>>,
    pub weekly_all: Option<f64>,
    pub weekly_all_resets_at: Option<DateTime<Utc>>,
    pub models: Vec<ModelWindow>,
    /// A linha de plano do topo, copiada como veio.
    pub plan: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum ProbeError {
    /// O Claude Code não está instalado em nenhum lugar conhecido.
    #[error("binário `claude` não encontrado — a sonda precisa dele")]
    NotInstalled,
    /// Perfil sem login. Não é erro para mostrar: quem chama segue em frente.
    #[error("sem login neste perfil")]
    NotSignedIn,
    /// Havia linha `Current …` e ela não se leu: formato novo, que a próxima
    /// versão precisa suportar.
    #[error("formato do /usage desconhecido")]
    Unrecognized,
    /// Estourou o prazo. Um processo travado não pode segurar a medição inteira.
    #[error("a sonda estourou o prazo")]
    TimedOut,
    #[error("não foi possível executar o claude: {0}")]
    Failed(String),
}

/// O que o processo devolveu.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProbeOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
}

/// Uma conta a medir, com o perfil já decidido pelo motor
/// (`RotationEngine::probe_config_dir`: conta ativa vai pelo perfil do grupo,
/// NUNCA pela casa).
#[derive(Clone, PartialEq, Debug)]
pub struct ProbeTarget {
    pub account_id: Id,
    pub label: String,
    pub email: String,
    pub dir: ConfigDir,
}

type Runner = Box<dyn Fn(&ConfigDir) -> Result<ProbeOutput, ProbeError> + Send + Sync>;

/// `CREATE_NO_WINDOW`: o app é um processo sem console, e um filho de console
/// piscaria uma janela preta a cada conta medida.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct ClaudeUsageProbe {
    runner: Runner,
}

impl ClaudeUsageProbe {
    /// Tempo de sobra para um processo frio numa máquina ocupada, curto o
    /// bastante para uma sonda travada não segurar as outras.
    pub const TIMEOUT: Duration = Duration::from_secs(45);

    /// Modo `--print` (sem o modo interativo e o diálogo de confiança),
    /// `--no-session-persistence` (sem um transcript por sondagem) e
    /// `--strict-mcp-config` sem `--mcp-config` (nenhum servidor MCP sobe). A
    /// telemetria fica LIGADA de propósito: `DISABLE_TELEMETRY` também fecha a
    /// consulta de feature flags, e a linha por modelo está atrás de uma delas.
    pub const ARGUMENTS: [&'static str; 4] = [
        "--print",
        "--no-session-persistence",
        "--strict-mcp-config",
        "/usage",
    ];

    /// Com o executor dado — injetável: o que vale testar é o que o texto
    /// significa, e subir o Claude Code de verdade pediria login e rede.
    pub fn new(
        runner: impl Fn(&ConfigDir) -> Result<ProbeOutput, ProbeError> + Send + Sync + 'static,
    ) -> Self {
        ClaudeUsageProbe {
            runner: Box::new(runner),
        }
    }

    /// A sonda de verdade, sobre o `claude` achado pelo resolvedor.
    pub fn system(command: ClaudeCommand, scratch_base: PathBuf) -> Self {
        Self::system_with_timeout(command, scratch_base, Self::TIMEOUT)
    }

    pub fn system_with_timeout(
        command: ClaudeCommand,
        scratch_base: PathBuf,
        timeout: Duration,
    ) -> Self {
        Self::new(move |dir| run(&command, dir, &scratch_base, timeout))
    }

    /// Mede um perfil. `now` entra por parâmetro porque o ano da data de reset
    /// depende dele.
    pub fn read(&self, dir: &ConfigDir, now: DateTime<Utc>) -> Result<Reading, ProbeError> {
        let output = (self.runner)(dir)?;
        Self::interpret(&output, now)
    }

    /// Mede uma conta e grava a amostra da sonda (origem `probe`, com as janelas
    /// por modelo e o carimbo delas). A escrita passa pela trava do motor: o
    /// sensor pode estar gravando a mesma conta, e a costura que preserva o
    /// bloco por modelo é ler-mudar-escrever.
    pub fn measure_into(
        &self,
        target: &ProbeTarget,
        usage_dir: &Path,
        lock_base: &Path,
        now: DateTime<Utc>,
    ) -> Result<Reading, ProbeError> {
        let reading = self.read(&target.dir, now)?;
        let sample = GroupUsageSample::new(
            target.dir.raw.clone(),
            Some(target.email.clone()),
            reading.session,
            reading.session_resets_at,
            reading.weekly_all,
            reading.weekly_all_resets_at,
            now,
            Some(ModelUsage::new(reading.models.clone(), now)),
            UsageOrigin::Probe,
        );
        let _lock = EngineLock::acquire(lock_base, EngineLock::WAIT);
        GroupUsageStore::write(&sample, &target.email, usage_dir)
            .map_err(|e| ProbeError::Failed(e.to_string()))?;
        Ok(reading)
    }

    /// O que a saída do processo significa. Código ≠ 0 é "sem login" (como no
    /// macOS); código 0 sem nenhuma linha `Current` também é — é o que um perfil
    /// deslogado imprime no Windows. Linha `Current` que não se lê é formato novo.
    pub fn interpret(output: &ProbeOutput, now: DateTime<Utc>) -> Result<Reading, ProbeError> {
        if output.exit_code != Some(0) {
            return Err(ProbeError::NotSignedIn);
        }
        match Self::parse(&output.stdout, now) {
            Err(ProbeError::Unrecognized) if !has_current_line(&output.stdout) => {
                Err(ProbeError::NotSignedIn)
            }
            other => other,
        }
    }

    /// Onde o `/usage` roda: uma pasta só pela vida da instalação. O Claude Code
    /// indexa transcripts pelo diretório de trabalho — uma pasta nova por chamada
    /// deixaria uma pasta de projeto nova a cada medição. Numa pasta limpa, o
    /// aviso de "workspace não confiável" (visto no spike) também não aparece.
    pub fn scratch_directory(base: &Path) -> io::Result<PathBuf> {
        let dir = base.join("probe-scratch");
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// As linhas que interessam; tudo abaixo delas é prosa sobre o consumo.
    pub fn parse(text: &str, now: DateTime<Utc>) -> Result<Reading, ProbeError> {
        let text = text.replace("\r\n", "\n");
        let (mut session, mut session_reset) = (None, None);
        let (mut weekly, mut weekly_reset) = (None, None);
        let mut models: Vec<ModelWindow> = Vec::new();

        for caps in line_regex().captures_iter(&text) {
            let Some(percent) = caps.get(3).and_then(|m| m.as_str().parse::<f64>().ok()) else {
                continue;
            };
            let fraction = percent / 100.0;
            // A data é opcional de propósito: perder uma porcentagem que leu
            // perfeitamente bem porque a redação da data mudou é a pior das duas
            // falhas.
            let reset = caps.get(4).and_then(|m| Self::reset_date(m.as_str(), now));

            if caps.get(1).is_some() {
                session = Some(fraction);
                session_reset = reset;
            } else if let Some(scope) = caps.get(2).map(|m| m.as_str()) {
                if scope.eq_ignore_ascii_case("all models") {
                    weekly = Some(fraction);
                    weekly_reset = reset;
                } else if !models.iter().any(|m| m.name == scope) {
                    models.push(ModelWindow::new(scope, fraction, reset));
                }
            }
        }

        if session.is_none() && weekly.is_none() && models.is_empty() {
            return Err(ProbeError::Unrecognized);
        }
        Ok(Reading {
            session,
            session_resets_at: session_reset,
            weekly_all: weekly,
            weekly_all_resets_at: weekly_reset,
            models,
            plan: Self::plan(&text),
        })
    }

    /// A linha de plano do topo, como impressa. Só as 3 primeiras linhas não
    /// vazias (mais abaixo "Max" aparece em prosa), e por palavra inteira — um
    /// aviso que fale de "projects" não vira plano "pro".
    pub fn plan(text: &str) -> Option<String> {
        let head: Vec<&str> = text
            .lines()
            .map(|l| l.trim_end_matches('\r'))
            .filter(|l| !l.trim().is_empty())
            .take(3)
            .collect();
        let head = head.join("\n");
        ["Max 20x", "Max 5x", "Pro", "Team", "subscription"]
            .iter()
            .find_map(|phrase| {
                Regex::new(&format!(r"(?i)\b{}\b", regex::escape(phrase)))
                    .ok()?
                    .find(&head)
                    .map(|m| m.as_str().to_string())
            })
    }

    /// `Sep 22, 8:40pm (America/Sao_Paulo)` (Windows) ou `Sep 18 at 7:29pm (…)`
    /// (macOS) → instante. A zona sai ANTES de ler am/pm (`America/…` começa com
    /// "Am"); zona inválida ou ausente = fuso local. Os minutos somem na hora
    /// cheia (`4am`). O ano não é impresso: vale o candidato (ano−1, ano, ano+1)
    /// mais perto de `now` — qualquer outra regra erra a virada do ano.
    pub fn reset_date(text: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let mut stamp = text.trim();
        let mut zone: Option<Tz> = None;
        if stamp.ends_with(')') {
            if let Some(open) = stamp.rfind('(') {
                zone = stamp[open + 1..stamp.len() - 1].trim().parse::<Tz>().ok();
                stamp = stamp[..open].trim();
            }
        }
        let caps = stamp_regex().captures(stamp)?;
        let month = month_number(&caps[1])?;
        let day: u32 = caps[2].parse().ok()?;
        let hour12: u32 = caps[3].parse().ok()?;
        let minute: u32 = match caps.get(4) {
            Some(m) => m.as_str().parse().ok()?,
            None => 0,
        };
        if !(1..=12).contains(&hour12) || minute > 59 {
            return None;
        }
        let pm = caps[5].eq_ignore_ascii_case("pm");
        let hour = match (hour12, pm) {
            (12, false) => 0,
            (12, true) => 12,
            (h, false) => h,
            (h, true) => h + 12,
        };
        match zone {
            Some(tz) => nearest(&tz, month, day, hour, minute, now),
            None => nearest(&Local, month, day, hour, minute, now),
        }
    }
}

fn line_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?m)^Current (?:(session)|week \(([^)]+)\)):\s*(\d+)%\s*used(?:\s*·\s*resets\s*(.+?))?\s*$",
        )
        .expect("regex da linha Current")
    })
}

fn stamp_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)^([a-z]{3,9})\.?\s+(\d{1,2})(?:,|\s+at)?\s+(\d{1,2})(?::(\d{2}))?\s*(am|pm)$",
        )
        .expect("regex da data de reset")
    })
}

fn has_current_line(text: &str) -> bool {
    text.lines().any(|l| l.trim_start().starts_with("Current "))
}

fn month_number(name: &str) -> Option<u32> {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let key = name.get(..3)?.to_ascii_lowercase();
    MONTHS.iter().position(|m| *m == key).map(|i| i as u32 + 1)
}

fn nearest<Z: TimeZone>(
    tz: &Z,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let year = now.with_timezone(tz).year();
    [year - 1, year, year + 1]
        .into_iter()
        .filter_map(|y| {
            let naive = NaiveDate::from_ymd_opt(y, month, day)?.and_hms_opt(hour, minute, 0)?;
            let local = match tz.from_local_datetime(&naive) {
                LocalResult::Single(t) => t,
                LocalResult::Ambiguous(first, _) => first,
                // Hora que não existe (início do horário de verão): a de depois.
                LocalResult::None => tz
                    .from_local_datetime(&(naive + chrono::Duration::hours(1)))
                    .earliest()?,
            };
            Some(local.with_timezone(&Utc))
        })
        .min_by_key(|d| (*d - now).num_seconds().abs())
}

/// Roda `claude --print … /usage` no perfil, com prazo.
fn run(
    command: &ClaudeCommand,
    dir: &ConfigDir,
    scratch_base: &Path,
    timeout: Duration,
) -> Result<ProbeOutput, ProbeError> {
    let scratch = ClaudeUsageProbe::scratch_directory(scratch_base)
        .map_err(|e| ProbeError::Failed(e.to_string()))?;
    // Direto (sem proxy/credencial alternativa), sem as variáveis de uma sessão
    // em volta, e com o perfil: no padrão a variável SAI (setá-la no caminho
    // padrão sobe deslogado).
    let env = ProviderEnv::with_var(
        ProviderEnv::without_nested_session(ProviderEnv::direct(std::env::vars_os())),
        "CLAUDE_CONFIG_DIR",
        dir.environment_value().map(OsStr::new),
    );
    let mut cmd = command.command();
    cmd.args(ClaudeUsageProbe::ARGUMENTS)
        .current_dir(&scratch)
        .env_clear()
        .envs(env)
        // Nunca um terminal: herdando o stdin, o `claude` esperaria uma entrada
        // que nunca vem. stderr descartado: um pipe que ninguém lê enche e trava.
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW);
    let mut child = cmd.spawn().map_err(|e| ProbeError::Failed(e.to_string()))?;

    // O stdout é lido aos pedaços numa thread: um neto que herde o pipe o
    // manteria aberto, e um `read_to_end` penduraria depois do fim do processo.
    let mut stdout = child.stdout.take().expect("stdout em pipe");
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match stdout.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let deadline = Instant::now() + timeout;
    let mut bytes = Vec::new();
    let status = loop {
        while let Ok(chunk) = rx.try_recv() {
            bytes.extend(chunk);
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill(); // TerminateProcess
                let _ = child.wait();
                break None;
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(e) => return Err(ProbeError::Failed(e.to_string())),
        }
    };
    let Some(status) = status else {
        return Err(ProbeError::TimedOut);
    };
    // O que ainda está no pipe, com uma folga curta.
    let grace = Instant::now() + Duration::from_secs(2);
    while let Some(left) = grace.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(chunk) => bytes.extend(chunk),
            Err(_) => break,
        }
    }
    Ok(ProbeOutput {
        exit_code: status.code(),
        stdout: String::from_utf8_lossy(&bytes).into_owned(),
    })
}
