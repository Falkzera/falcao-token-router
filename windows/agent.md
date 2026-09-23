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
- `crates/gauge-mark/` — a marca (o anel): bandeja e ícone do app do mesmo desenho, por pixel.
- `docs/PLATFORM.md` — os fatos do Windows verificados (inglês), o mapa macOS → Windows e as
  diferenças deliberadas.
- `scripts/test.ps1` — a verificação (fmt + clippy + testes + checagens do front), igual na CI.
- `scripts/build.ps1` — o instalador NSIS (o `router.exe` como sidecar), conferido; igual na CI.
- `app/` — o app Tauri v2 + Svelte 5 (`src-tauri/` é membro deste workspace).

## Estado
- **Fase 3 (sensor)** e **Fase 4 (núcleo + CLI)** feitas em TDD (os testes do router no Swift
  portados — Engine, Store, Launcher, Probe, SessionRegistry — mais as regressões do Windows),
  clippy `-D warnings` e fmt limpos.
- **Fase 5 (app Tauri)** em fatias, todas feitas: 5.0 o que o app pede ao núcleo
  (`terminal_report`, grupo criado dedicado, login estranho no `~\.claude`, contas exclusivas,
  medição em três passos, casa pendente descartada); 5.1 o anel (`gauge-mark`) e o esqueleto
  Tauri + Svelte no workspace, no `test.ps1` e na CI; 5.2 a bandeja (anel, tooltip com
  procedência, laço de rotação); 5.3 o flyout; 5.4 a janela Grupos (cartões, ordem, confirmações,
  integração de terminal por shell) e Ajustes (abrir no login, barra de tarefas); 5.5 o login
  oficial por ConPTY (adicionar e relogar, com os estados do macOS e o tempo esgotado visível).
  Conferido no navegador com o backend simulado (os dois idiomas e temas) e no app do sandbox.
- Próximas: fase 6 (instalador NSIS com o `router.exe` como sidecar, `build.ps1`, README em
  inglês) e fase 7 (ponta a ponta com contas reais, grupo dedicado).

## Regras (herdadas do repo + combinadas)
- Comentários e `agent.md` em **pt-BR**; identificadores em inglês; docs/README em inglês.
- Nada de chamada de rede; nenhum tipo com campo de token (credencial = blob opaco).
- Fixtures anonimizadas: `conta1@exemplo.com`, `C:\Users\exemplo`, org `Acme`, `win32:exemplo-pc`.
- Formatos de arquivo idênticos ao macOS — é o contrato do `docs/PORTING.md`.
