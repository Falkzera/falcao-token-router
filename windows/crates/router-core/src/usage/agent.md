# usage — uso (≙ Sources/CCUsageCore/Usage)

## Arquivos
- `claude_binary.rs` — o resolvedor ÚNICO do `claude`: `ROUTER_CLAUDE_BIN` →
  `%USERPROFILE%\.local\bin\claude.exe` → `PATH` (`claude.exe`, `claude.cmd`) → `%APPDATA%\npm`.
  Shim do npm LIDO (vira `node` + `cli.js` ou o `.exe` do pacote — nunca `cmd.exe`); cópia do
  Claude Desktop e aliases do WindowsApps excluídos. `ClaudeCommand::command()` dá o `Command`.
- `usage_percent.rs` — `UsagePercent::value/text`: fração 0–1 → inteiro, **num lugar só**
  (arredonda meio para longe do zero, igual ao `.rounded()` do Swift). Evita o painel
  truncar enquanto a status line arredonda.
- `claude_usage_probe.rs` — nesta fatia só a struct `ModelWindow {name, percent, resetsAt?}`,
  que a amostra guarda. O parser do `/usage` e a execução do processo entram na Fase 4.

## Pendências (Fase 4 — a sonda)
- `parse`/`resetDate`: o Windows imprime a data com **vírgula** (`MMM d, h:mma` / `MMM d, ha`),
  não `'at'` como o macOS; `·`=U+00B7; CRLF; deslogado = exit 0 **sem** linhas `Current`
  (guiar pela presença das linhas, não pelo exit code). Zona IANA via chrono-tz.
