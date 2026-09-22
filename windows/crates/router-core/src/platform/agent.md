# platform — o específico do Windows

Funções pequenas atrás das quais mora o que é da plataforma.

## Arquivos
- `atomic_write.rs` — `write_atomic`: grava num temporário na mesma pasta e renomeia por
  cima. O `rename` é atômico no Win 10+ e, por ser de arquivo recém-escrito, deixa **mtime
  novo** — o que a credencial vai exigir (o Claude Code releva a credencial quando o mtime
  muda). Nunca `CopyFile`, que preservaria o mtime da origem.

## Pendências (Fase 4+)
- `credential_store` (escrita com retry em violação de compartilhamento, leitura dupla),
  `links` (junction/symlink + Developer Mode), `process_liveness` (FILETIME), `short_path`,
  `known_folders` (Documentos via OneDrive), `git_bash`, `profile_append` (bytes/encoding).
