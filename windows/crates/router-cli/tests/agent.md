# router-cli/tests — a CLI ponta a ponta

Roda o `router.exe` de verdade num sandbox: base (`ROUTER_APP_SUPPORT`), home
(`USERPROFILE`), `APPDATA` e `PATH` só com o System32 — o `claude` real da máquina
nunca é achado; o `fake-claude` entra por `ROUTER_CLAUDE_BIN`. `CLAUDE_CONFIG_DIR` é
removido (o teste pode rodar dentro de uma sessão do Claude Code, com o perfil real).

## Arquivos
- `common/mod.rs` — `Sandbox` (`router(args)`, `command(programa)`, `records()` do fake) e
  `world(n)`: um grupo dedicado "Trabalho" com `n` contas logadas (credencial + identidade).
- `statusline_cli.rs` — o sensor: grava e imprime; sem `rate_limits` não grava; stdin que nunca
  fecha sai rápido; prazo sem entrada.
- `cli_tests.rs` — `is-group` (mudo), `launch` (troca, ambiente limpo, argumentos com e sem `--`,
  código de saída, sensor e junction plantados, sai da conta cheia), `rotate` (espelha antes de
  trocar), `measure` (saída, amostra `probe`, ativa pelo grupo), sensor com `--profile`, `doctor`.
- `shell_tests.rs` — `shell.ps1` EXECUTADO no Windows PowerShell 5.1 e `shell.sh` no Git Bash com
  `PATH` mínimo: roteia, repassa o código, encadeia a função anterior, sem recursão ao recarregar,
  e avisa alto quando o `router.exe` sumiu.

## Padrões
- Precisa do `fake-claude` compilado: rodar com `cargo test --workspace` (ou `scripts\test.ps1`).
