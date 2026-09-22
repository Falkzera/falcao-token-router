//! Provedor de IA cujo CLI o app sabe rotacionar. A v1 traz só o `.anthropic`
//! (Claude Code). O tipo existe agora porque decide o formato do arquivo de
//! configuração — cada conta e grupo carrega o provedor a que pertence.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    #[default]
    Anthropic,
}

impl Provider {
    /// Nome que aparece na UI. Não traduzível: é marca.
    pub fn display_name(&self) -> &'static str {
        match self {
            Provider::Anthropic => "Claude",
        }
    }
}
