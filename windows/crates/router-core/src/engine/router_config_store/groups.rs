//! Grupos: criar, renomear, padrão, limiar, ordem, apagar.
//!
//! Parte do `impl RouterConfigStore` — ver o `mod.rs` ao lado.

use super::*;

impl RouterConfigStore {
    // MARK: - Grupos (o que a UI chama)

    /// Cria um grupo. O primeiro vira o padrão (`~\.claude`); os seguintes ganham
    /// perfil dedicado. Qual é o padrão pode ser mudado depois.
    pub fn add_group(&mut self, name: &str) -> AccountGroup {
        let is_first = self.config.groups.is_empty();
        self.add_group_with(name, is_first)
    }

    /// Cria um grupo decidindo JÁ se ele usa o `~\.claude`. O app pergunta antes
    /// quando o `~\.claude` tem um login que o router não conhece: um grupo que
    /// nasce padrão teria esse login trocado (com backup) assim que a primeira
    /// conta entrasse — o laço de rotação ativa a primeira conta de um grupo
    /// sem ativa. Padrão novo tira o posto de quem o tinha (no máximo um).
    pub fn add_group_with(&mut self, name: &str, as_default: bool) -> AccountGroup {
        let mut group = AccountGroup::new(name, ConfigDir::dedicated(String::new()));
        group.config_dir = self
            .paths
            .group_config_dir(group.id, as_default, &self.home);
        if as_default {
            for i in 0..self.config.groups.len() {
                if self.config.groups[i].config_dir.is_default {
                    let id = self.config.groups[i].id;
                    self.config.groups[i].config_dir =
                        self.paths.group_config_dir(id, false, &self.home);
                }
            }
        }
        self.config.groups.push(group.clone());
        self.save();
        group
    }

    /// O login que o `~\.claude` tem hoje e que não é de nenhuma conta do
    /// router — o que uma ativação num grupo padrão substituiria (a guarda faz
    /// backup antes, mas o usuário tem de saber). `None` sem login lá (identidade
    /// sem credencial é o que sobra de um logout) ou com login de conta conhecida.
    pub fn foreign_default_login(&self) -> Option<String> {
        let identity = self.login.login_result(&ConfigDir::standard(&self.home))?;
        let known = self
            .config
            .accounts
            .iter()
            .any(|a| a.identity.email.eq_ignore_ascii_case(&identity.email));
        (!known).then_some(identity.email)
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
        let exclusive = self.exclusive_account_ids(id);
        self.config.groups.remove(i);
        for account_id in exclusive {
            self.remove_account(account_id);
        }
        self.save();
    }

    /// As contas que só existem neste grupo — as que apagá-lo leva junto (com a
    /// credencial). É o número que a confirmação da UI diz; o macOS mostrava o
    /// total do grupo, contando também a compartilhada, que fica.
    pub fn exclusive_account_ids(&self, group_id: Id) -> Vec<Id> {
        let Some(i) = self.group_index(group_id) else {
            return Vec::new();
        };
        let elsewhere: HashSet<Id> = self
            .config
            .groups
            .iter()
            .filter(|g| g.id != group_id)
            .flat_map(|g| g.account_ids.iter().copied())
            .collect();
        self.config.groups[i]
            .account_ids
            .iter()
            .copied()
            .filter(|a| !elsewhere.contains(a))
            .collect()
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
}
