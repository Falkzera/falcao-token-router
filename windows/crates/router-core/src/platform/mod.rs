//! O que é específico da plataforma (Windows), atrás de funções pequenas: escrita
//! atômica de arquivo, e — nas próximas fases — links, liveness, caminhos curtos.

pub mod atomic_write;
pub mod console;
pub mod git_bash;
pub mod host;
pub mod job;
pub mod json_file;
pub mod known_folders;
pub mod links;
pub mod named_mutex;
pub mod paths;
pub mod powershell;
pub mod process;
pub mod process_times;
pub mod profile_append;
pub mod short_path;
pub mod ui_language;
