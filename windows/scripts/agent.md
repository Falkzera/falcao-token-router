# scripts — verificação do porte

- `test.ps1` — a verificação que a CI roda: `cargo fmt --check`, `cargo clippy --workspace
  --all-targets -D warnings` e `cargo test --workspace`. Acha o `cargo` do rustup se ele não
  estiver no `PATH`. Rodar a partir de qualquer pasta.

## Pendências (Fase 6)
- `build.ps1` (release + `router.exe` como sidecar do instalador NSIS).
