//! O que é específico da plataforma (Windows), atrás de funções pequenas: escrita
//! atômica de arquivo, e — nas próximas fases — links, liveness, caminhos curtos.

pub mod atomic_write;
pub mod named_mutex;
pub mod paths;
