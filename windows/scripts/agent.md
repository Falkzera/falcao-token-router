# scripts — verificação do porte

- `test.ps1` — a verificação que a CI roda: `cargo fmt --check`, `cargo clippy --workspace
  --all-targets -D warnings`, o build do `fake-claude`, `cargo test --workspace` e, no front,
  `svelte-check` + `check-strings`. Acha o `cargo` do rustup se ele não estiver no `PATH`.
  Rodar a partir de qualquer pasta.
- 23/09/2026: o `fake-claude` é compilado ANTES dos testes — o `cargo test` não gera o .exe de
  um pacote sem testes de integração, e numa máquina limpa (a CI) os testes da CLI e do login
  o procurariam em vão (aqui ele existia de builds anteriores; conferido removendo o .exe).

## Pendências (Fase 6)
- `build.ps1` (release + `router.exe` como sidecar do instalador NSIS).
