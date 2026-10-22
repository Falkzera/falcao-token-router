//! Contas: reservar casa, concluir login e relogin, remover, apelidar.
//!
//! Parte do `impl RouterConfigStore` — ver o `mod.rs` ao lado.

use super::*;

impl RouterConfigStore {
    // MARK: - Contas

    /// Reserva a casa de uma conta nova (o perfil onde ela fará login), sem
    /// lançar nada. O resultado é confirmado depois por `finish_pending_login`.
    pub fn new_account_home(&self) -> (ConfigDir, Id) {
        let id = Id::new();
        let home = self.paths.account_home(id);
        let _ = fs::create_dir_all(home.path());
        (home, id)
    }

    /// Apaga a casa reservada de um login que não virou conta (cancelado, ou que
    /// voltou duplicado e será refeito numa casa nova): ela pode ter recebido uma
    /// credencial de verdade, e ninguém mais a usaria. Nunca a casa de uma conta
    /// registrada — o relogin usa a MESMA casa, e cancelar um relogin não pode
    /// apagar a conta — nem pasta fora de `<base>\accounts\`. Devolve se apagou.
    pub fn discard_pending_home(&self, home: &ConfigDir, account_id: Id) -> bool {
        let registered = self
            .config
            .accounts
            .iter()
            .any(|a| a.id == account_id || a.home.raw.eq_ignore_ascii_case(&home.raw));
        let dir = home.path();
        if registered
            || home.is_default
            || !is_strictly_inside(&dir, &self.paths.base.join("accounts"))
        {
            return false;
        }
        self.credentials
            .delete(&self.engine.credential_location_of(home));
        fs::remove_dir_all(&dir).is_ok() || !dir.exists()
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

    /// Relogin que voltou com OUTRA conta: o `claude auth login` já gravou o
    /// login dela na casa desta, e ali ele faria "Usar" servir a outra conta com
    /// o nome desta (o macOS deixava). A credencial estranha sai — a conta fica
    /// sem login, o estado honesto, que a tela manda relogar; se ela está ativa
    /// num grupo, o próximo espelho devolve à casa a do grupo. Só age quando a
    /// casa tem mesmo OUTRA identidade. Devolve se apagou.
    pub fn discard_wrong_relogin(&self, account_id: Id) -> bool {
        let Some(account) = self.config.account(account_id) else {
            return false;
        };
        match self.login.login_result(&account.home) {
            Some(found) if found.email != account.identity.email => {
                self.credentials
                    .delete(&self.engine.credential_location_of(&account.home));
                true
            }
            _ => false,
        }
    }

    /// Remove a conta do registro **e apaga a credencial dela** (e a casa).
    ///
    /// Se ela serve um grupo agora, o grupo passa para a próxima conta (ver
    /// `hand_over`) — a troca a quente de sempre, então a sessão viva segue, só
    /// que atendida pela próxima. Até 22/10/2026 a cópia do grupo ficava
    /// intocada: o grupo seguia servido pela conta removida, e a tela o mostrava
    /// sem conta em uso.
    ///
    /// Sem próxima (era a única, ou nenhuma pode assumir), a cópia do grupo
    /// segue intocada: apagá-la derrubaria a sessão viva atendida por ela. A
    /// pasta da casa só é apagada se for uma que o router criou
    /// (`<base>\accounts\…`) — nunca uma pasta qualquer de um config editado.
    pub fn remove_account(&mut self, account_id: Id) {
        let Some(account) = self.config.account(account_id).cloned() else {
            return;
        };
        let location = self.engine.home_credential_location(&account);
        // Apurado ANTES de a conta sair do registro: depois, a identidade no
        // perfil do grupo não casa com conta nenhuma.
        let served: Vec<Id> = self
            .config
            .groups
            .iter()
            .filter(|g| {
                self.engine
                    .active_account(g, &self.config)
                    .is_some_and(|a| a.id == account_id)
            })
            .map(|g| g.id)
            .collect();

        self.config.accounts.retain(|a| a.id != account_id);
        for group in &mut self.config.groups {
            group.account_ids.retain(|id| *id != account_id);
        }
        self.save();

        for group_id in served {
            self.hand_over(group_id);
        }

        self.credentials.delete(&location);
        let home = account.home.path();
        if is_strictly_inside(&home, &self.paths.base.join("accounts")) {
            let _ = fs::remove_dir_all(&home);
        }
        self.refresh_usage();
    }

    /// Passa o grupo cuja conta ativa acabou de ser removida para a primeira
    /// de `RotationEngine::successors` que aceitar a ativação — quem não pode
    /// (sem credencial na casa, ativa noutro grupo) é pulada. Com a conta já
    /// fora do registro, o `activate` não espelha a que sai: a casa dela vai
    /// ser apagada. Se outro processo já pôs uma conta do registro no grupo
    /// (a rotação da CLI), fica a escolha dele.
    fn hand_over(&self, group_id: Id) {
        let Some(group) = self
            .group_index(group_id)
            .map(|i| self.config.groups[i].clone())
        else {
            return;
        };
        let _lock = EngineLock::acquire(&self.paths.base, EngineLock::WAIT);
        if self.engine.active_account(&group, &self.config).is_some() {
            return;
        }
        for candidate in self
            .engine
            .successors(&group, &self.config, &self.usage_snapshot)
        {
            if self
                .engine
                .activate(candidate, &group, &self.config)
                .is_ok()
            {
                return;
            }
        }
    }

    pub fn set_nickname(&mut self, account_id: Id, nickname: Option<&str>) {
        if let Some(account) = self.config.accounts.iter_mut().find(|a| a.id == account_id) {
            account.nickname = nickname.filter(|n| !n.is_empty()).map(String::from);
            self.save();
        }
    }
}
