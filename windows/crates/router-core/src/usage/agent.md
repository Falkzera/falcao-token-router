# usage — uso (≙ macos/Sources/CCUsageCore/Usage)

## Arquivos
- `claude_binary.rs` — o resolvedor ÚNICO do `claude`: `ROUTER_CLAUDE_BIN` →
  `%USERPROFILE%\.local\bin\claude.exe` → `PATH` (`claude.exe`, `claude.cmd`) → `%APPDATA%\npm`.
  Shim do npm LIDO (vira `node` + `cli.js` ou o `.exe` do pacote — nunca `cmd.exe`); cópia do
  Claude Desktop e aliases do WindowsApps excluídos. `ClaudeCommand::command()` dá o `Command`.
- `usage_percent.rs` — `UsagePercent::value/text`: fração 0–1 → inteiro, **num lugar só**
  (arredonda meio para longe do zero, igual ao `.rounded()` do Swift). Evita o painel
  truncar enquanto a status line arredonda.
- `claude_usage_probe.rs` — ≙ `ClaudeUsageProbe`: `ModelWindow`, `Reading`, `ProbeError`,
  `ProbeTarget`. `parse` (linhas `Current …`, `·`=U+00B7, CRLF normalizado; `session` → 5h,
  `all models` → 7d, outro escopo → por modelo, a 1ª ocorrência vence), `reset_date` (zona IANA
  via `chrono-tz` tirada ANTES de am/pm, senão fuso local; `MMM d, h:mma`/`MMM d, ha` do Windows
  e `MMM d 'at' …` do macOS; ano = candidato mais perto de `now`), `plan` (3 primeiras linhas,
  palavra inteira), `interpret` (código ≠ 0 ou saída SEM linha `Current` = `NotSignedIn`; linha
  `Current` ilegível = `Unrecognized`), o executor (`--print --no-session-persistence
  --strict-mcp-config /usage`, cwd `probe-scratch`, ambiente direto sem `CLAUDE_CODE*`,
  `CLAUDE_CONFIG_DIR` só no dedicado, stdin nulo, stderr descartado, `CREATE_NO_WINDOW`, prazo
  45 s → `TerminateProcess`, stdout lido aos pedaços) e `measure_into` (grava a amostra `probe`
  sob a trava do motor).

## Decisões
- 22/09/2026 (spike): deslogado no Windows sai com código **0** e só o resumo do `--print` —
  "sem login" é a ausência de linhas `Current`, não o código de saída.
- 22/09/2026: telemetria LIGADA de propósito na sonda (a linha por modelo está atrás de uma
  feature flag que `DISABLE_TELEMETRY` fecha) — igual ao macOS.
- O plano é casado por palavra inteira: um aviso que fale de "projects" não vira "pro" (no macOS
  a busca é por substring; na pasta limpa da sonda o aviso não aparece, mas não custa).
