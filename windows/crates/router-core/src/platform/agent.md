# platform — o específico do Windows

Funções pequenas atrás das quais mora o que é da plataforma.

## Arquivos
- `atomic_write.rs` — `write_atomic`: grava num temporário na mesma pasta e renomeia por
  cima. O `rename` é atômico no Win 10+ e, por ser de arquivo recém-escrito, deixa **mtime
  novo** — o que a credencial exige (o Claude Code relê a credencial quando o mtime muda).
  Nunca `CopyFile`, que preservaria o mtime da origem. `retrying`/`read_retrying`/
  `remove_retrying`: nova tentativa (~0,6 s) nos erros 5/32/33/1224 — outro processo com o
  arquivo aberto sem compartilhar (antivírus, indexador, o próprio Claude Code).
- `json_file.rs` — `edit_object`: edição cirúrgica de JSON-objeto (`.claude.json`,
  `settings.json`): preserva ordem e chaves, recuo 2, **recusa** arquivo ilegível (nunca troca
  por `{}`), ausente/vazio = objeto novo.
- `named_mutex.rs` — `acquire(nome, prazo)`: mutex nomeado (`CreateMutexW`), trava entre
  processos. Guarda `!Send` (a posse é da thread); mutex abandonado por dono morto é assumido.
- `process_times.rs` — `probe(pid)` (`OpenProcess` + `GetExitCodeProcess` + `GetProcessTimes`:
  `Missing`/`Running(início)`/`Denied` = existe mas é de outro dono) e FILETIME ↔ data.
- `host.rs` — `local_host_names`: nome DNS (`GetComputerNameExW`) e `%COMPUTERNAME%`, para o
  `pidDomain` (`win32:<host>` em minúsculas, visto no spike).
- `paths.rs` — comparação de caminho do Windows sem tocar o disco (`normalized`,
  `is_strictly_inside`: sem caixa, `/` = `\`, `..` desqualifica).
- `short_path.rs` — nome 8.3 (`GetShortPathNameW`) de um caminho que existe; `None` se o 8.3
  está desligado no volume (ainda sobra espaço no nome).
- `git_bash.rs` — `find_git_bash`: o bash que o Claude Code usa para a status line, na ordem
  dele (`CLAUDE_CODE_GIT_BASH_PATH` → Program Files → Program Files (x86) → git do PATH).
- `known_folders.rs` — `documents_dir`: a Documentos real (`SHGetKnownFolderPath`), que com o
  OneDrive não é `%USERPROFILE%\Documents` — é onde moram os `$PROFILE`.
- `profile_append.rs` — `append_block`: acrescenta ao perfil do shell **em bytes**, na
  codificação do BOM (UTF-16LE/BE, UTF-8; sem BOM o bloco é ASCII, igual em ANSI) e no fim de
  linha do arquivo; modo append (segue link, não troca o arquivo); arquivo ilegível = erro.
- `process.rs` — `run_with_timeout`: comando curto com prazo (stdout por thread, mata ao
  estourar), sem console (`CREATE_NO_WINDOW` — do app cada consulta piscaria uma janela).
- `links.rs` — `junction` (pastas, sem privilégio), `symlink_file` (flag
  `ALLOW_UNPRIVILEGED_CREATE`; sem Developer Mode falha com 1314), `is_junction`/`is_symlink`,
  `developer_mode_enabled` (registro `AppModelUnlock`, só para dica na UI/`doctor`).

## Decisões
- 22/09/2026: hardlink NÃO para `history.jsonl` — o binário 2.1.280 reescreve o arquivo comum
  na poda de retenção (e pula link), e o hardlink divergiria em silêncio.

## Pendências (Fase 4+)
- console (Ctrl+C no `launch`, na CLI). A política de execução do PowerShell mora em
  `engine::terminal_report` desde a fase 5.
