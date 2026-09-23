//! O motor: modelos de conta/grupo, formato da amostra, leitor de uso, adapter e
//! caminhos. Espelha `Sources/CCUsageCore/Engine` do macOS.

pub mod account_login_service;
pub mod account_model;
pub mod anthropic_adapter;
pub mod config_dir;
pub mod credential_store;
pub mod default_profile_guard;
pub mod engine_lock;
pub mod group_model;
pub mod group_usage;
pub mod group_usage_reader;
pub mod profile_sharing;
pub mod provider;
pub mod provider_env;
pub mod rotation_engine;
pub mod router_config_store;
pub mod router_paths;
pub mod session_launcher;
pub mod session_registry;
pub mod shell_integration;
