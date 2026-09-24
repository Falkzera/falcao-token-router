//! Motor do Falcão Token Router — porte Windows.
//!
//! Espelha o `CCUsageCore` do macOS (a parte do router), lendo e escrevendo os
//! MESMOS arquivos: `config.json` e `usage/<email>.json`. As regras de negócio
//! são portadas 1:1, com os mesmos nomes e os mesmos comentários de "porquê".
//!
//! Esta primeira fatia é o **sensor**: modelos, formato da amostra, leitor de uso
//! e `usage_percent`. O resto do motor (rotação, credencial, store) vem depois.

pub mod ids;
pub mod time_fmt;

pub mod engine;
pub mod platform;
pub mod statusline;
pub mod usage;

// Reexports de conveniência — a superfície pública fica plana como no Swift.
pub use engine::account_model::{Account, AccountIdentity};
pub use engine::config_dir::ConfigDir;
pub use engine::group_model::{AccountGroup, RouterConfig};
pub use engine::group_usage::{GroupUsageSample, GroupUsageStore, ModelUsage, UsageOrigin};
pub use engine::group_usage_reader::{AccountUsage, GroupUsageReader, UsageWindow};
pub use engine::provider::Provider;
pub use ids::Id;
pub use usage::claude_usage_probe::ModelWindow;
pub use usage::usage_percent::UsagePercent;
