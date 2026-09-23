# fake-claude — um `claude` de mentira (só testes)

Binário que os testes de integração da CLI usam no lugar do `claude` (via
`ROUTER_CLAUDE_BIN`). Nunca empacotado (`publish = false`, fora do instalador).

## Arquivos
- `main.rs` — registra cada execução em `FAKE_CLAUDE_RECORD` (argumentos, pasta, perfil e os
  NOMES das variáveis sensíveis que chegaram); `FAKE_CLAUDE_EXIT` é o código de saída;
  `FAKE_CLAUDE_SLEEP_MS` atrasa; no `/usage`, perfil com `.credentials.json` imprime
  `FAKE_CLAUDE_USAGE` (ou a captura anonimizada do Windows) e sem credencial imprime o resumo de
  deslogado com código 0 — como o real; `--version` imprime `2.1.280 (Claude Code)`.
