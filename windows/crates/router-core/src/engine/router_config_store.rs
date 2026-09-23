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

    // MARK: - Grupos (o que a UI chama)

    /// Cria um grupo. O primeiro vira o padrão (`~\.claude`); os seguintes ganham
    /// perfil dedicado. Qual é o padrão pode ser mudado depois.
    pub fn add_group(&mut self, name: &str) -> AccountGroup {
        let is_first = self.config.groups.is_empty();
        let mut group = AccountGroup::new(name, ConfigDir::dedicated(String::new()));
        group.config_dir = self.paths.group_config_dir(group.id, is_first, &self.home);
        self.config.groups.push(group.clone());
        self.save();
        group
    }

    pub fn rename_group(&mut self, id: Id, name: &str) {
        if let Some(i) = self.group_index(id) {
            self.config.groups[i].name = name.to_string();
            self.save();
        }
    }

    /// Limiar entre 50% e 100%.
    pub fn set_threshold(&mut self, id: Id, percent: f64) {
        if let Some(i) = self.group_index(id) {
            self.config.groups[i].threshold_percent = percent.clamp(50.0, 100.0);
            self.save();
        }
    }

    pub fn set_auto_rotate(&mut self, id: Id, on: bool) {
        if let Some(i) = self.group_index(id) {
            self.config.groups[i].auto_rotate = on;
            self.save();
        }
    }

    /// Reordena as contas de um grupo — a ordem é a preferência de rotação.
    /// Ignora id estranho; uma conta do grupo que a lista pedida esqueceu vai
    /// para o fim, na ordem de antes (o macOS a tirava do grupo).
    pub fn reorder_accounts(&mut self, group_id: Id, ordered: &[Id]) {
        let Some(i) = self.group_index(group_id) else {
            return;
        };
        let current = self.config.groups[i].account_ids.clone();
        let mut next: Vec<Id> = Vec::with_capacity(current.len());
        for id in ordered {
            if current.contains(id) && !next.contains(id) {
                next.push(*id);
            }
        }
        for id in current {
            if !next.contains(&id) {
                next.push(id);
            }
        }
        self.config.groups[i].account_ids = next;
        self.save();
    }

    /// Apaga o grupo e leva junto as contas que só existiam nele (órfãs não
    /// aparecem em tela nenhuma) — pelo mesmo caminho da remoção avulsa, para a
    /// credencial de cada uma sair junto.
    pub fn remove_group(&mut self, id: Id) {
        let Some(i) = self.group_index(id) else {
            return;
        };
        let elsewhere: HashSet<Id> = self
            .config
            .groups
            .iter()
            .filter(|g| g.id != id)
            .flat_map(|g| g.account_ids.iter().copied())
            .collect();
        let exclusive: Vec<Id> = self.config.groups[i]
            .account_ids
            .iter()
            .copied()
            .filter(|a| !elsewhere.contains(a))
            .collect();
        self.config.groups.remove(i);
        for account_id in exclusive {
            self.remove_account(account_id);
        }
        self.save();
    }

    /// Tira o status de padrão de todos: nenhum grupo passa a usar o `~\.claude`,
    /// e o router deixa de tocar lá.
    pub fn clear_default(&mut self) {
        for i in 0..self.config.groups.len() {
            if self.config.groups[i].config_dir.is_default {
                let id = self.config.groups[i].id;
                self.config.groups[i].config_dir =
                    self.paths.group_config_dir(id, false, &self.home);
            }
        }
        self.save();
    }

    /// Torna um grupo o padrão (`~\.claude`), tirando de quem era. No máximo um.
    pub fn make_default(&mut self, id: Id) {
        for i in 0..self.config.groups.len() {
            let group_id = self.config.groups[i].id;
            let should_be = group_id == id;
            if self.config.groups[i].config_dir.is_default != should_be {
                self.config.groups[i].config_dir =
                    self.paths.group_config_dir(group_id, should_be, &self.home);
            }
        }
        self.save();
    }

    // MARK: - Contas

    /// Reserva a casa de uma conta nova (o perfil onde ela fará login), sem
    /// lançar nada. O resultado é confirmado depois por `finish_pending_login`.
    pub fn new_account_home(&self) -> (ConfigDir, Id) {
        let id = Id::new();
        let home = self.paths.account_home(id);
        let _ = fs::create_dir_all(home.path());
        (home, id)
    }

    /// Confere se um login pendente terminou. Distingue "ainda não" de "veio a
    /// mesma conta de novo" (o caso comum quando o navegador não pediu conta).
    pub fn finish_pending_login(
        &mut self,
        home: &ConfigDir,
        account_id: Id,
        into: Id,
    ) -> LoginOutcome {
        let Some(identity) = self.login.login_result(home) else {
            return LoginOutcome::Pending;
        };
        let in_group: Vec<Id> = self
            .group_index(into)
            .map(|i| self.config.groups[i].account_ids.clone())
            .unwrap_or_default();
        if self
            .config
            .accounts
            .iter()
            .any(|a| a.identity.email == identity.email && in_group.contains(&a.id))
        {
            return LoginOutcome::Duplicate {
                email: identity.email,
            };
        }
        let account = Account {
            id: account_id,
            provider: Provider::Anthropic,
            identity,
            home: home.clone(),
            nickname: None,
        };
        self.config.accounts.push(account.clone());
        if let Some(i) = self.group_index(into) {
            self.config.groups[i].account_ids.push(account.id);
        }
        self.save();
        LoginOutcome::Added(account)
    }

    /// Confere se o relogin de uma conta existente terminou. Relogin reusa a
    /// MESMA casa, então sucesso = casa com identidade + credencial de novo.
    pub fn finish_relogin(&mut self, account_id: Id) -> ReloginOutcome {
        let Some(i) = self.config.accounts.iter().position(|a| a.id == account_id) else {
            return ReloginOutcome::Pending;
        };
        let Some(identity) = self.login.login_result(&self.config.accounts[i].home) else {
            return ReloginOutcome::Pending;
        };
        let expected = self.config.accounts[i].identity.email.clone();
        if identity.email != expected {
            return ReloginOutcome::WrongAccount {
                expected,
                got: identity.email,
            };
        }
        self.config.accounts[i].identity = identity; // tier/organização podem ter mudado
        self.save();
        let account = self.config.accounts[i].clone();
        // Conta ativa num grupo: o grupo guarda a credencial MORTA que motivou o
        // relogin — a nova vai por cima, senão a sessão segue no "Login expired".
        {
            let _lock = EngineLock::acquire(&self.paths.base, EngineLock::WAIT);
            for group in &self.config.groups {
                if self
                    .engine
                    .active_account(group, &self.config)
                    .is_some_and(|a| a.id == account_id)
                {
                    self.engine.push_home_to_group(&account, group);
                }
            }
        }
        self.refresh_usage();
        ReloginOutcome::Renewed(account)
    }

    /// Remove a conta do registro **e apaga a credencial dela** (e a casa).
    ///
    /// A cópia do GRUPO não é tocada de propósito, mesmo quando é esta conta que
    /// o serve: pode haver sessão viva atendida por ela agora. Ela é sobrescrita
    /// na próxima ativação. A pasta da casa só é apagada se for uma que o router
    /// criou (`<base>\accounts\…`) — nunca uma pasta qualquer de um config editado.
    pub fn remove_account(&mut self, account_id: Id) {
        let Some(account) = self.config.account(account_id).cloned() else {
            return;
        };
        let location = self.engine.home_credential_location(&account);

        self.config.accounts.retain(|a| a.id != account_id);
        for group in &mut self.config.groups {
            group.account_ids.retain(|id| *id != account_id);
        }
        self.save();

        self.credentials.delete(&location);
        let home = account.home.path();
        if is_strictly_inside(&home, &self.paths.base.join("accounts")) {
            let _ = fs::remove_dir_all(&home);
        }
    }

    pub fn set_nickname(&mut self, account_id: Id, nickname: Option<&str>) {
        if let Some(account) = self.config.accounts.iter_mut().find(|a| a.id == account_id) {
            account.nickname = nickname.filter(|n| !n.is_empty()).map(String::from);
            self.save();
        }
    }

    // MARK: - Rotação (ações que tocam credencial)

    /// A conta que serve um grupo agora.
    pub fn active_account(&self, group: &AccountGroup) -> Option<Account> {
        self.engine.active_account(group, &self.config).cloned()
    }

    /// Troca manual. Erro vira `last_error`; sucesso relê o quadro na hora, para
    /// o indicador da UI se mover.
    pub fn activate(&mut self, account: &Account, group: &AccountGroup) {
        let result = {
            let _lock = EngineLock::acquire(&self.paths.base, EngineLock::WAIT);
            self.engine.activate(account, group, &self.config)
        };
        match result {
            Ok(_) => {
                self.last_error = None;
                self.refresh_usage();
            }
            Err(e) => self.last_error = Some(StoreError::ActivateFailed(e)),
        }
    }

    /// As contas de um grupo com o perfil por onde medir cada uma — decidido
    /// AQUI, com o config na mão: conta ativa vai pelo perfil do grupo, nunca
    /// pela casa (sondar a casa de uma conta ativa derruba a sessão viva).
    pub fn probe_targets(&self, group: &AccountGroup) -> Vec<ProbeTarget> {
        self.config
            .accounts_in(group)
            .into_iter()
            .map(|account| ProbeTarget {
                account_id: account.id,
                label: account.label().to_string(),
                email: account.identity.email.clone(),
                dir: self.engine.probe_config_dir(account, &self.config),
            })
            .collect()
    }

    /// Mede as contas de um grupo com a **sonda ativa**, uma por vez, e publica.
    ///
    /// É o que o sensor passivo não consegue: conta ociosa nunca serviu mensagem
    /// (sem amostra, "pronta") e o limite POR MODELO não chega no `rate_limits`.
    /// Sob demanda, não no laço: cada conta custa um processo e uma requisição.
    /// Síncrona: o app a chama fora da thread da interface.
    pub fn measure_accounts(
        &mut self,
        group: &AccountGroup,
        probe: Option<&ClaudeUsageProbe>,
    ) -> MeasureSummary {
        let Some(probe) = probe else {
            self.last_error = Some(StoreError::ProbeUnavailable);
            return MeasureSummary::default();
        };
        let mut summary = MeasureSummary::default();
        let usage_dir = self.paths.usage_dir();
        for target in self.probe_targets(group) {
            match probe.measure_into(&target, &usage_dir, &self.paths.base, Utc::now()) {
                Ok(_) => summary.measured += 1,
                Err(_) => summary.failed += 1,
            }
        }
        self.last_error = (summary.failed > 0).then_some(StoreError::ProbeFailures(summary.failed));
        self.refresh_usage();
        summary
    }

    /// Relê o uso das amostras, a conta ativa e as sessões vivas de cada grupo, e
    /// publica.
    pub fn refresh_usage(&mut self) {
        let detail = self.usage.detail_by_account(&self.config, Utc::now());
        self.usage_snapshot = detail.iter().map(|(id, u)| (*id, u.fraction)).collect();
        self.usage_detail = detail;
        self.usage_sampled_at = self
            .usage
            .samples_by_account(&self.config)
            .into_iter()
            .map(|(id, s)| (id, s.sampled_at))
            .collect();
        self.active_by_group = self
            .config
            .groups
            .iter()
            .filter_map(|g| {
                self.engine
                    .active_account(g, &self.config)
                    .map(|a| (g.id, a.id))
            })
            .collect();
        self.live_sessions = self
            .config
            .groups
            .iter()
            .map(|g| (g.id, (self.session_reader)(&g.config_dir)))
            .filter(|(_, sessions)| !sessions.is_empty())
            .collect();
    }

    /// Uma volta da rotação automática em todos os grupos: espelha a ativa (a
    /// casa recebe o token vivo) e troca se ela passou do limiar e há destino.
    /// Usa o `usage_snapshot` mais recente.
    pub fn rotate_all(&mut self) {
        let groups = self.config.groups.clone();
        for group in &groups {
            {
                let _lock = EngineLock::acquire(&self.paths.base, EngineLock::WAIT);
                self.engine.mirror_active(group, &self.config);
            }
            let Some(target) = self
                .engine
                .rotation_target(group, &self.config, &self.usage_snapshot)
                .cloned()
            else {
                continue;
            };
            self.activate(&target, group);
        }
    }

    // MARK: - Integração com o terminal

    pub fn set_router_path(&mut self, path: Option<PathBuf>) {
        self.router_path = path;
    }

    pub fn router_path(&self) -> Option<&Path> {
        self.router_path.as_deref()
    }

    /// O `shell.ps1` que os perfis do PowerShell carregam.
    pub fn powershell_script_path(&self) -> PathBuf {
        self.paths.base.join("shell.ps1")
    }

    /// O `shell.sh` que o `~\.bashrc` do Git Bash carrega.
    pub fn bash_script_path(&self) -> PathBuf {
        self.paths.base.join("shell.sh")
    }

    /// A status line que o perfil de um grupo deve ter, para o shell que o
    /// Claude Code vai usar.
    pub fn expected_status_line(&self, group: &AccountGroup, shell: StatusShell) -> Option<String> {
        let router = self.router_path.as_deref()?;
        Some(ShellIntegration::status_line_for_profile(
            router,
            &group.config_dir,
            shell,
        ))
    }

    /// Escreve os scripts, planta a status line e o compartilhamento em cada
    /// perfil de grupo, e acrescenta a linha nos perfis de shell. Idempotente.
    /// A parte do Git Bash só entra quando há Git Bash (`shell == Bash`).
    pub fn install_shell_integration(
        &mut self,
        targets: &ShellTargets,
        shell: StatusShell,
    ) -> Result<(), StoreError> {
        let Some(router) = self.router_path.clone() else {
            self.last_error = Some(StoreError::RouterPathUnknown);
            return Err(StoreError::RouterPathUnknown);
        };
        let (ps1, sh) = (self.powershell_script_path(), self.bash_script_path());
        let mut failures: Vec<String> = Vec::new();

        if let Err(e) = ShellIntegration::write_scripts(&router, &ps1, &sh) {
            failures.push(e.to_string());
        }
        let home = ConfigDir::standard(&self.home);
        for group in &self.config.groups {
            if let Some(command) = self.expected_status_line(group, shell) {
                if let Err(e) = ShellIntegration::install_status_line(&command, &group.config_dir) {
                    failures.push(e.to_string());
                }
            }
            ProfileSharing::link(
                &group.config_dir,
                &home,
                self.config.share_history,
                &self.paths.base,
            );
        }
        let ps_line = ShellIntegration::powershell_source_line(&ps1);
        for profile in &targets.powershell_profiles {
            if let Err(e) = ShellIntegration::ensure_in_profile(profile, &ps_line, "\r\n") {
                failures.push(format!("{}: {e}", profile.display()));
            }
        }
        if shell == StatusShell::Bash {
            let sh_line = ShellIntegration::bash_source_line(&sh);
            if let Err(e) = ShellIntegration::ensure_in_profile(&targets.bashrc, &sh_line, "\n") {
                failures.push(format!("{}: {e}", targets.bashrc.display()));
            }
            if let Err(e) = ShellIntegration::ensure_bash_profile(targets) {
                failures.push(e.to_string());
            }
        }

        if failures.is_empty() {
            self.last_error = None;
            Ok(())
        } else {
            let error = StoreError::IntegrationFailed(failures.join("; "));
            self.last_error = Some(error.clone());
            Err(error)
        }
    }

    /// A integração instalada aponta para um binário que não é mais este (app
    /// movido ou reinstalado noutro lugar), ou algum perfil de grupo está sem a
    /// status line certa. O modo de falha é silencioso — `claude trabalho` cai
    /// no `claude` puro, na conta errada — por isso a cura é automática.
    /// Não instalada não é obsoleta: é ausente.
    pub fn integration_is_stale(&self, shell: StatusShell) -> bool {
        let Some(router) = self.router_path.as_deref() else {
            return false;
        };
        let ps1 = self.powershell_script_path();
        let Ok(script) = read_retrying(&ps1) else {
            return false;
        };
        let router_text = router.to_string_lossy().replace('\'', "''");
        if !String::from_utf8_lossy(&script).contains(router_text.as_str()) {
            return true;
        }
        self.config.groups.iter().any(|group| {
            self.expected_status_line(group, shell)
                .is_some_and(|cmd| ShellIntegration::status_line_is_stale(&cmd, &group.config_dir))
        })
    }

    /// Reinstala quando ficou obsoleta. Devolve `true` se mexeu em algo.
    pub fn heal_shell_integration(&mut self, targets: &ShellTargets, shell: StatusShell) -> bool {
        self.integration_is_stale(shell) && self.install_shell_integration(targets, shell).is_ok()
    }
}
