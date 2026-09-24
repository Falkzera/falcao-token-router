//! O dono da configuração do produto, e a ponte entre a UI e o motor
//! (≙ `RouterConfigStore.swift`).
//!
//! A UI edita grupos e contas por aqui; cada mudança persiste o `config.json`.
//! As ações que tocam credencial (ativar, rotacionar, empurrar pós-relogin)
//! passam pelo `RotationEngine`, sob a trava entre processos. Sem UI e sem
//! runtime assíncrono: exercitável em teste, como o resto do crate.
//!
//! Diferenças deliberadas do macOS: a home do usuário é injetada (os testes do
//! macOS usavam a real, escondida); os erros são fatos tipados (`StoreError`),
//! e o texto fica com a UI; um `config.json` ilegível nunca é sobrescrito — é
//! guardado de lado no primeiro `save`; reordenar não derruba conta esquecida;
//! remover conta só apaga pasta que o router criou.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};

use super::account_login_service::AccountLoginService;
use super::account_model::Account;
use super::anthropic_adapter::AnthropicAdapter;
use super::config_dir::ConfigDir;
use super::credential_store::CredentialStore;
use super::engine_lock::EngineLock;
use super::group_model::{AccountGroup, RouterConfig};
use super::group_usage_reader::{AccountUsage, GroupUsageReader};
use super::profile_sharing::ProfileSharing;
use super::provider::{Provider, ProviderAdapter};
use super::rotation_engine::{RotationEngine, RotationError};
use super::router_paths::RouterPaths;
use super::session_registry::{LiveSession, SessionRegistry};
use super::shell_integration::{ShellIntegration, ShellTargets, StatusShell};
use crate::ids::Id;
use crate::platform::atomic_write::{read_retrying, write_atomic};
use crate::platform::paths::is_strictly_inside;
use crate::usage::claude_usage_probe::{ClaudeUsageProbe, ProbeTarget};

// O `impl RouterConfigStore` é dividido por responsabilidade, um arquivo cada.
// Um tipo só continua sendo a fachada que a UI chama — o que muda é que cada
// motivo de mudança tem seu arquivo, e nenhum passa de 600 linhas.
mod accounts;
mod groups;
mod rotation;
mod terminal;

/// Última falha de uma ação, para a UI mostrar (com o texto do catálogo dela).
#[derive(Clone, PartialEq, Debug, thiserror::Error)]
pub enum StoreError {
    #[error("não foi possível salvar a configuração: {0}")]
    SaveFailed(String),
    #[error("não foi possível trocar de conta: {0}")]
    ActivateFailed(RotationError),
    /// O app não disse onde está o `router.exe`: sem ele não há integração.
    #[error("não foi possível localizar o binário do router")]
    RouterPathUnknown,
    /// Alguma peça da integração de terminal não foi gravada.
    #[error("não foi possível instalar a integração: {0}")]
    IntegrationFailed(String),
    /// Sem `claude` instalado não há sonda.
    #[error("binário `claude` não encontrado — a sonda precisa dele")]
    ProbeUnavailable,
    /// Contas que não responderam à sonda (quase sempre: precisam de Relogar).
    #[error("{0} conta(s) não responderam à sonda — veja se precisam de Relogar")]
    ProbeFailures(usize),
}

/// O saldo de uma medição com a sonda.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MeasureSummary {
    pub measured: usize,
    pub failed: usize,
}

/// Uma medição planejada: as contas com o perfil de cada uma e onde gravar.
/// Roda sem o store — a sonda leva segundos por conta.
#[derive(Clone, Debug)]
pub struct MeasurePlan {
    pub targets: Vec<ProbeTarget>,
    pub usage_dir: PathBuf,
    pub base: PathBuf,
}

impl MeasurePlan {
    /// Sonda as contas uma por vez, gravando cada amostra (origem `probe`).
    pub fn run(&self, probe: &ClaudeUsageProbe) -> MeasureSummary {
        let mut summary = MeasureSummary::default();
        for target in &self.targets {
            match probe.measure_into(target, &self.usage_dir, &self.base, Utc::now()) {
                Ok(_) => summary.measured += 1,
                Err(_) => summary.failed += 1,
            }
        }
        summary
    }
}

/// O resultado de checar um login pendente.
#[derive(Clone, PartialEq, Debug)]
pub enum LoginOutcome {
    /// Ainda não terminou; continue observando.
    Pending,
    /// Entrou uma conta nova; foi adicionada ao grupo.
    Added(Account),
    /// O login trouxe uma conta que já está no grupo — quase sempre porque o
    /// navegador ainda estava logado nela.
    Duplicate { email: String },
}

/// O resultado de checar um relogin pendente.
#[derive(Clone, PartialEq, Debug)]
pub enum ReloginOutcome {
    Pending,
    /// A credencial nova entrou, na mesma conta. Se ela estava ativa num grupo,
    /// o grupo já recebeu a credencial nova.
    Renewed(Account),
    /// O navegador logou OUTRA conta. Nada do registro muda.
    WrongAccount {
        expected: String,
        got: String,
    },
}

/// De onde vêm as sessões vivas de um perfil.
type SessionReader = Box<dyn Fn(&ConfigDir) -> Vec<LiveSession> + Send + Sync>;

/// O que se achou no `config.json` ao abrir.
enum Loaded {
    Missing,
    Ready(RouterConfig),
    Unreadable,
}

pub struct RouterConfigStore {
    config: RouterConfig,
    /// Uso 0–1 por conta, lido das amostras.
    usage_snapshot: HashMap<Id, f64>,
    /// Quando a amostra de cada conta foi colhida — a idade que a UI mostra.
    usage_sampled_at: HashMap<Id, DateTime<Utc>>,
    /// O mesmo uso, com a janela e a origem de onde o número veio.
    usage_detail: HashMap<Id, AccountUsage>,
    /// A conta ativa de cada grupo (grupo → conta).
    active_by_group: HashMap<Id, Id>,
    /// As sessões do Claude Code vivas em cada grupo (grupo → sessões). O
    /// registro é POR PERFIL: diz qual sessão roda em qual grupo, e por qual
    /// conta ela é atendida — a resposta que o `/status` não dá.
    live_sessions: HashMap<Id, Vec<LiveSession>>,
    /// De onde vêm as sessões de um perfil. Injetável: os testes do macOS liam o
    /// `~/.claude/sessions` real sem perceber.
    session_reader: SessionReader,
    last_error: Option<StoreError>,
    /// O `config.json` existia e não pôde ser lido: no primeiro `save` ele é
    /// guardado de lado, nunca sobrescrito.
    unreadable_on_disk: bool,
    /// O `router.exe` que a integração e a status line apontam. Setado pelo app
    /// ao iniciar; `None` fora dele.
    router_path: Option<PathBuf>,
    paths: RouterPaths,
    /// A home do usuário (`%USERPROFILE%`), de onde sai o perfil padrão.
    home: String,
    engine: RotationEngine,
    credentials: Arc<dyn CredentialStore>,
    login: AccountLoginService,
    usage: GroupUsageReader,
}

impl RouterConfigStore {
    pub fn new(
        paths: RouterPaths,
        home: String,
        credentials: Arc<dyn CredentialStore>,
        adapters: Vec<Arc<dyn ProviderAdapter>>,
    ) -> Self {
        let adapter = adapters
            .first()
            .cloned()
            .unwrap_or_else(|| Arc::new(AnthropicAdapter));
        let (config, unreadable_on_disk) = match Self::load(&paths) {
            Loaded::Missing => (RouterConfig::default(), false),
            Loaded::Ready(config) => (config, false),
            Loaded::Unreadable => (RouterConfig::default(), true),
        };
        let mut store = RouterConfigStore {
            config,
            usage_snapshot: HashMap::new(),
            usage_sampled_at: HashMap::new(),
            usage_detail: HashMap::new(),
            active_by_group: HashMap::new(),
            live_sessions: HashMap::new(),
            session_reader: Box::new(SessionRegistry::live_sessions),
            last_error: None,
            unreadable_on_disk,
            router_path: None,
            usage: GroupUsageReader::new(paths.usage_dir()),
            engine: RotationEngine::new(credentials.clone(), adapters),
            login: AccountLoginService::new(adapter, credentials.clone()),
            credentials,
            paths,
            home,
        };
        // Publica o quadro completo já na abertura: a bandeja é desenhada antes
        // de qualquer laço rodar, e sem a conta ativa ela não diria quem serve.
        store.refresh_usage();
        store
    }

    // MARK: - Leitura do estado

    pub fn config(&self) -> &RouterConfig {
        &self.config
    }

    pub fn paths(&self) -> &RouterPaths {
        &self.paths
    }

    pub fn usage_snapshot(&self) -> &HashMap<Id, f64> {
        &self.usage_snapshot
    }

    pub fn usage_sampled_at(&self) -> &HashMap<Id, DateTime<Utc>> {
        &self.usage_sampled_at
    }

    pub fn usage_detail(&self) -> &HashMap<Id, AccountUsage> {
        &self.usage_detail
    }

    pub fn active_by_group(&self) -> &HashMap<Id, Id> {
        &self.active_by_group
    }

    pub fn last_error(&self) -> Option<&StoreError> {
        self.last_error.as_ref()
    }

    /// O usuário dispensou o aviso: o erro só volta com uma nova falha.
    pub fn clear_last_error(&mut self) {
        self.last_error = None;
    }

    pub fn live_sessions(&self) -> &HashMap<Id, Vec<LiveSession>> {
        &self.live_sessions
    }

    /// Quantas sessões vivas um grupo tem agora.
    pub fn session_count(&self, group_id: Id) -> usize {
        self.live_sessions.get(&group_id).map_or(0, Vec::len)
    }

    /// Troca de onde vêm as sessões (testes) e republica o quadro.
    pub fn with_session_reader(
        mut self,
        reader: impl Fn(&ConfigDir) -> Vec<LiveSession> + Send + Sync + 'static,
    ) -> Self {
        self.session_reader = Box::new(reader);
        self.refresh_usage();
        self
    }

    // MARK: - Persistência

    fn load(paths: &RouterPaths) -> Loaded {
        match read_retrying(&paths.config_file()) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Loaded::Missing,
            Err(_) => Loaded::Unreadable,
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Loaded::Ready)
                .unwrap_or(Loaded::Unreadable),
        }
    }

    fn try_save(&mut self) -> io::Result<()> {
        fs::create_dir_all(&self.paths.base)?;
        if self.unreadable_on_disk {
            // Guarda o ilegível de lado: pode ser a configuração inteira do
            // usuário com um byte estragado, recuperável à mão.
            let aside = self.paths.base.join(format!(
                "config.unreadable-{}.json",
                Utc::now().format("%Y%m%dT%H%M%SZ")
            ));
            match fs::rename(self.paths.config_file(), aside) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
            self.unreadable_on_disk = false;
        }
        // Sem segredo aqui (a credencial mora no perfil), mas há e-mail e plano
        // de cada conta; a pasta herda a ACL do `%LOCALAPPDATA%`, só do usuário.
        let bytes = serde_json::to_vec(&self.config).map_err(io::Error::other)?;
        write_atomic(&self.paths.config_file(), &bytes)
    }

    fn save(&mut self) {
        if let Err(e) = self.try_save() {
            self.last_error = Some(StoreError::SaveFailed(e.to_string()));
        }
    }

    fn group_index(&self, id: Id) -> Option<usize> {
        self.config.groups.iter().position(|g| g.id == id)
    }
}
