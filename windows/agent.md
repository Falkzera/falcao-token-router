# windows/ — porte Windows do Falcão Token Router

Porte do router (só do router; o medidor herdado fica de fora) para **Rust + Tauri
v2**, lendo e escrevendo os MESMOS arquivos que o app macOS: `config.json` e
`usage/<email>.json`. Nada aqui muda a versão macOS — tudo vive sob `windows/` (e
um `.github/workflows/windows.yml` filtrado por caminho).

## Estrutura
- `Cargo.toml` — workspace (resolver 2, edition 2021, rust-version 1.89).
- `rust-toolchain.toml` — canal stable, alvo `x86_64-pc-windows-msvc`.
- `crates/router-core/` — o motor (modelos, sensor, leitor de uso). Sem I/O de rede.
- `crates/router-cli/` — a CLI `router` (por ora só `statusline`).
- `scripts/test.ps1` — a verificação (fmt + clippy + testes), igual na CI.
- `app/` (fase 5, ainda não existe) — o app Tauri.

## Estado
- **Fase 3 (sensor) feita e validada:** `router statusline` lê `rate_limits` do
  stdin (com prazo, sem esperar EOF), grava a amostra por conta (só se houver
  janela) e imprime a linha colorida. Testes portados do macOS + regressões Windows.
- Próximo: Fase 4 — núcleo e CLI completos (rotação, credencial em arquivo,
  `launch`/`is-group`/`rotate`/`doctor`/`measure`) em TDD.

## Regras (herdadas do repo + combinadas)
- Comentários e `agent.md` em **pt-BR**; identificadores em inglês; docs/README em inglês.
- Nada de chamada de rede; nenhum tipo com campo de token (credencial = blob opaco).
- Fixtures anonimizadas: `conta1@exemplo.com`, `C:\Users\exemplo`, org `Acme`, `win32:exemplo-pc`.
- Formatos de arquivo idênticos ao macOS — é o contrato do `docs/PORTING.md`.
