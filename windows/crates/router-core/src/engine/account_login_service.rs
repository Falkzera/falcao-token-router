//! Observa o resultado do **fluxo oficial** de login (≙ `AccountLoginService`).
//!
//! Os termos exigem que o login termine no fluxo da própria Anthropic: nada de
//! webview embutida, nada de coletar senha. O app não faz login — roda o binário
//! oficial num pty, num perfil isolado — e este serviço só **lê o desfecho no
//! disco**: identidade + credencial na casa da conta.

use std::sync::Arc;

use super::account_model::AccountIdentity;
use super::config_dir::ConfigDir;
use super::credential_store::CredentialStore;
use super::provider::ProviderAdapter;

pub struct AccountLoginService {
    adapter: Arc<dyn ProviderAdapter>,
    credentials: Arc<dyn CredentialStore>,
}

impl AccountLoginService {
    pub fn new(adapter: Arc<dyn ProviderAdapter>, credentials: Arc<dyn CredentialStore>) -> Self {
        AccountLoginService {
            adapter,
            credentials,
        }
    }

    /// A identidade que apareceu num perfil depois do login, ou `None` se ainda
    /// não terminou. Só conta como pronto quando as DUAS coisas existem: a
    /// identidade no `.claude.json` e a credencial (completa) no perfil — uma
    /// sem a outra é login pela metade.
    pub fn login_result(&self, home: &ConfigDir) -> Option<AccountIdentity> {
        let identity = self.adapter.identity(home)?;
        self.credentials
            .exists(&self.adapter.credential_location(home))
            .then_some(identity)
    }
}
