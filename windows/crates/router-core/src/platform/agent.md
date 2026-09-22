# platform — o específico do Windows

Funções pequenas atrás das quais mora o que é da plataforma.

## Arquivos
- `atomic_write.rs` — `write_atomic`: grava num temporário na mesma pasta e renomeia por
  cima. O `rename` é atômico no Win 10+ e, por ser de arquivo recém-escrito, deixa **mtime
  novo** — o que a credencial exige (o Claude Code relê a credencial quando o mtime muda).
  Nunca `CopyFile`, que preservaria o mtime da origem. `retrying`/`read_retrying`/
  `remove_retrying`: nova tentativa (~0,6 s) nos erros 5/32/33/1224 — outro processo com o
  arquivo aberto sem compartilhar (antivírus, indexador, o próprio Claude Code).
- `named_mutex.rs` — `acquire(nome, prazo)`: mutex nomeado (`CreateMutexW`), trava entre
  processos. Guarda `!Send` (a posse é da thread); mutex abandonado por dono morto é assumido.
- `process_times.rs` — `probe(pid)` (`OpenProcess` + `GetExitCodeProcess` + `GetProcessTimes`:
  `Missing`/`Running(início)`/`Denied` = existe mas é de outro dono) e FILETIME ↔ data.
- `host.rs` — `local_host_names`: nome DNS (`GetComputerNameExW`) e `%COMPUTERNAME%`, para o
  `pidDomain` (`win32:<host>` em minúsculas, visto no spike).
- `paths.rs` — comparação de caminho do Windows sem tocar o disco (`normalized`,
  `is_strictly_inside`: sem caixa, `/` = `\`, `..` desqualifica).

## Pendências (Fase 4+)
- `links` (junction/symlink + Developer Mode), `short_path`, `known_folders` (Documentos via OneDrive), `git_bash`, `profile_append`
  (bytes/encoding), console (Ctrl+C no `launch`).
