//! Rotação e medição — as ações que tocam credencial.
//!
//! Parte do `impl RouterConfigStore` — ver o `mod.rs` ao lado.

use super::*;

impl RouterConfigStore {
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
            self.measure_unavailable();
            return MeasureSummary::default();
        };
        let summary = self.measure_plan(group).run(probe);
        self.finish_measure(summary);
        summary
    }

    /// O que medir num grupo, decidido com o config na mão. É o primeiro dos
    /// três passos que o app usa para não segurar o store enquanto a sonda roda
    /// (segundos por conta): planejar (com o store) → `MeasurePlan::run` (sem ele)
    /// → `finish_measure` (com ele).
    pub fn measure_plan(&self, group: &AccountGroup) -> MeasurePlan {
        MeasurePlan {
            targets: self.probe_targets(group),
            usage_dir: self.paths.usage_dir(),
            base: self.paths.base.clone(),
        }
    }

    /// Publica o saldo de uma medição: o erro (contas que não responderam) e o
    /// quadro relido com as amostras novas.
    pub fn finish_measure(&mut self, summary: MeasureSummary) {
        self.last_error = (summary.failed > 0).then_some(StoreError::ProbeFailures(summary.failed));
        self.refresh_usage();
    }

    /// Sem `claude` instalado não há sonda — um erro com nome, para a UI.
    pub fn measure_unavailable(&mut self) {
        self.last_error = Some(StoreError::ProbeUnavailable);
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
}
