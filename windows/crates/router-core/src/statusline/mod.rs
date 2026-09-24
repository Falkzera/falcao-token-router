//! A status line dos grupos: o que o `router statusline` imprime depois de
//! gravar a amostra (`view`) e a escolha do usuário sobre ela (`choice`). Fica
//! no núcleo porque a CLI e o app (os Ajustes, com a prévia) usam o mesmo
//! código.

pub mod choice;
pub mod command;
pub mod session;
pub mod view;
