# windows/ — porte Windows do Falcão Token Router

Porte do router (só do router; o medidor herdado fica de fora) para **Rust + Tauri
v2**, lendo e escrevendo os MESMOS arquivos que o app macOS: `config.json` e
`usage/<email>.json`. Nada aqui muda a versão macOS — tudo vive sob `windows/` (e
um `.github/workflows/windows.yml` filtrado por caminho).

## Estrutura
- `Cargo.toml` — workspace (resolver 2, edition 2021, rust-version 1.89) e as dependências comuns.
- `rust-toolchain.toml` — canal stable, alvo `x86_64-pc-windows-msvc`.
- `crates/router-core/` — o motor (modelos, credencial em arquivo, rotação, store, sessões,
  integração de terminal, sonda). Sem UI, sem rede.
- `crates/router-cli/` — a CLI `router` (`statusline`, `launch`, `is-group`, `rotate`, `measure`,
  `doctor`).
- `crates/fake-claude/` — `claude` de mentira para os testes de integração (nunca empacotado).
- `docs/PLATFORM.md` — os fatos do Windows verificados (inglês), o mapa macOS → Windows e as
  diferenças deliberadas.
- `scripts/test.ps1` — a verificação (fmt + clippy + testes), igual na CI.
- `app/` (fase 5, ainda não existe) — o app Tauri.

## Estado
- **Fase 3 (sensor)** e **Fase 4 (núcleo + CLI)** feitas em TDD: 210 testes (os do router no
  Swift portados — Engine, Store, Launcher, Probe, SessionRegistry — mais as regressões do
  Windows), clippy `-D warnings` e fmt limpos.
- Próximo: Fase 5 — o app Tauri (bandeja, janela Grupos/Ajustes, login por ConPTY, laço de 180 s).

## Regras (herdadas do repo + combinadas)
- Comentários e `agent.md` em **pt-BR**; identificadores em inglês; docs/README em inglês.
- Nada de chamada de rede; nenhum tipo com campo de token (credencial = blob opaco).
- Fixtures anonimizadas: `conta1@exemplo.com`, `C:\Users\exemplo`, org `Acme`, `win32:exemplo-pc`.
- Formatos de arquivo idênticos ao macOS — é o contrato do `docs/PORTING.md`.
