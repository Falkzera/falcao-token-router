# app/src-tauri — o crate Rust do app

Membro do workspace de `windows/` (um `Cargo.lock`, um `target/`): o `test.ps1` o formata, o
passa no clippy e roda os testes dele junto com o resto.

## Arquivos
- `Cargo.toml` — `falcao-token-router` (lib `falcao_token_router_lib`); tauri 2.11 com
  `tray-icon`; plugins single-instance, autostart, opener, clipboard-manager (usados só do Rust).
- `build.rs` — `tauri_build::build()`: embute o `icons/icon.ico` no .exe e gera `gen/` (ignorado).
- `tauri.conf.json` — `identifier` = `com.synqo.falcao-token-router` (o bundle id do macOS);
  `productName` ASCII sem espaço (`FalcaoTokenRouter`); nenhuma janela declarada (o Rust as
  cria); CSP fechada; NSIS como alvo (configurado na fase 6).
- `capabilities/default.json` — as janelas `home` e `flyout` com `core:default` (eventos e o
  básico da janela).
- `icons/` — GERADOS pelo `icongen` do `gauge-mark` (não editar à mão).
- `src/` — o código (ver o `agent.md` de lá).
