//! Um diretório de configuração do Claude Code — um perfil.
//!
//! Guarda a **string crua** exportada como `CLAUDE_CONFIG_DIR` (no macOS ela vira
//! o hash do item de chaveiro; no Windows a credencial é arquivo, então não há
//! hash — mas o formato do `config.json` mantém `{raw, isDefault}` idêntico).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDir {
    /// A string exatamente como vai para o ambiente. No perfil padrão é o caminho
    /// de `<home>\.claude`; nos outros, o caminho do diretório do grupo.
    pub raw: String,

    /// `true` quando é o perfil padrão — o que o `claude` usa sem nenhuma
    /// variável de ambiente.
    ///
    /// Não é cosmético: setar `CLAUDE_CONFIG_DIR` para o caminho padrão **não** é
    /// o mesmo que não setar — com a variável presente a sessão sobe deslogada
    /// (confirmado no macOS; a checagem isolada no Windows fica para a
    /// implementação). O padrão se distingue por não exportar variável nenhuma.
    pub is_default: bool,
}

impl ConfigDir {
    /// O perfil padrão, `<home>\.claude`. Nunca exporta `CLAUDE_CONFIG_DIR`.
    pub fn standard(home: &str) -> Self {
        ConfigDir {
            raw: format!("{home}/.claude"),
            is_default: true,
        }
    }

    /// Um perfil dedicado, num caminho qualquer. Exporta `CLAUDE_CONFIG_DIR`.
    pub fn dedicated(path: impl Into<String>) -> Self {
        ConfigDir {
            raw: path.into(),
            is_default: false,
        }
    }

    /// O caminho do diretório em si.
    pub fn path(&self) -> PathBuf {
        PathBuf::from(&self.raw)
    }

    /// O `.claude.json` deste perfil.
    ///
    /// Assimetria confirmada no disco (macOS e Windows): no perfil padrão mora
    /// **ao lado** do diretório (`<home>\.claude.json`, não
    /// `<home>\.claude\.claude.json`); num perfil dedicado, mora **dentro**
    /// (`<dir>\.claude.json`).
    pub fn global_config_path(&self) -> PathBuf {
        let dir = self.path();
        if self.is_default {
            match dir.parent() {
                Some(parent) => parent.join(".claude.json"),
                None => PathBuf::from(".claude.json"),
            }
        } else {
            dir.join(".claude.json")
        }
    }

    /// A variável de ambiente que uma sessão neste perfil precisa — ou `None` no
    /// padrão, que não exporta nada.
    pub fn environment_value(&self) -> Option<&str> {
        if self.is_default {
            None
        } else {
            Some(&self.raw)
        }
    }
}
