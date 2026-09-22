# platform — o específico do Windows

Funções pequenas atrás das quais mora o que é da plataforma.

## Arquivos
- `atomic_write.rs` — `write_atomic`: grava num temporário na mesma pasta e renomeia por
  cima. O `rename` é atômico no Win 10+ e, por ser de arquivo recém-escrito, deixa **mtime
  novo** — o que a credencial exige (o Claude Code relê a credencial quando o mtime muda).
  Nunca `CopyFile`, que preservaria o mtime da origem. `retrying`/`read_retrying`/
  `remove_retrying`: nova tentativa (~0,6 s) nos erros 5/32/33/1224 — outro processo com o
  arquivo aberto sem compartilhar (antivírus, indexador, o próprio Claude Code).

## Pendências (Fase 4+)
- `credential_store` (usa o `write_atomic`/`read_retrying` daqui),
  `links` (junction/symlink + Developer Mode), `process_liveness` (FILETIME), `short_path`,
  `known_folders` (Documentos via OneDrive), `git_bash`, `profile_append` (bytes/encoding).
