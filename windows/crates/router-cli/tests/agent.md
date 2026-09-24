# router-cli/tests — a CLI ponta a ponta

Roda o `router.exe` de verdade num sandbox: base (`ROUTER_APP_SUPPORT`), home
(`USERPROFILE`), `APPDATA` e `PATH` só com o System32 — o `claude` real da máquina
nunca é achado; o `fake-claude` entra por `ROUTER_CLAUDE_BIN`. `CLAUDE_CONFIG_DIR` é
removido (o teste pode rodar dentro de uma sessão do Claude Code, com o perfil real).

## Arquivos
- `common/mod.rs` — `Sandbox` (`router(args)`, `command(programa)`, `records()` do fake) e
  `world(n)`: um grupo dedicado "Trabalho" com `n` contas logadas (credencial + identidade).
- `statusline_cli.rs` — o sensor: grava e imprime; sem `rate_limits` não grava; stdin que nunca
  fecha sai rápido; prazo sem entrada. A escolha: itens tirados somem; ilegível = a completa;
  modo comando imprime a linha do usuário (o `fake-claude statusline-echo`) a partir do mesmo
  JSON e com o EOF (e a linha sem quebra no fim não se perde); comando que falha cai na linha do
  app com os itens; o próprio router como comando não entra em laço.
- `cli_tests.rs` — `is-group` (mudo), `launch` (troca, ambiente limpo, argumentos com e sem `--`,
  código de saída, sensor e junction plantados, sai da conta cheia), `rotate` (espelha antes de
  trocar), `measure` (saída, amostra `probe`, ativa pelo grupo), sensor com `--profile`, `doctor`
  (inclusive a linha vazia aceita, a escolha nomeada e o comando do usuário testado).
- `shell_tests.rs` — `shell.ps1` EXECUTADO no Windows PowerShell 5.1 e `shell.sh` no Git Bash com
  `PATH` mínimo: roteia, repassa o código, encadeia a função anterior, sem recursão ao recarregar,
  e avisa alto quando o `router.exe` sumiu.

## Padrões
- Precisa do `fake-claude` compilado: rodar com `cargo test --workspace` (ou `scripts\test.ps1`).
