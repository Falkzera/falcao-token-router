# fake-claude — um `claude` de mentira (só testes)

Binário que os testes de integração da CLI usam no lugar do `claude` (via
`ROUTER_CLAUDE_BIN`). Nunca empacotado (`publish = false`, fora do instalador).

## Arquivos
- `main.rs` — registra cada execução em `FAKE_CLAUDE_RECORD` (argumentos, pasta, perfil e os
  NOMES das variáveis sensíveis que chegaram); `FAKE_CLAUDE_EXIT` é o código de saída;
  `FAKE_CLAUDE_SLEEP_MS` atrasa; no `/usage`, perfil com `.credentials.json` imprime
  `FAKE_CLAUDE_USAGE` (ou a captura anonimizada do Windows) e sem credencial imprime o resumo de
  deslogado com código 0 — como o real; `--version` imprime `2.1.280 (Claude Code)`; o
  `auth login` imprime o que o 2.1.280 imprime (o link num hyperlink OSC 8 quando a saída é
  terminal, `&login_hint=<e-mail>` com `--email`) e segue `FAKE_CLAUDE_LOGIN`: `ok:<e-mail>`
  (grava credencial e identidade no perfil e diz "Login successful."), `code:<e-mail>` (o mesmo
  depois de ler `a#b` do stdin; outro código dá "Invalid code"), `quiet:<e-mail>`, `nodisk`,
  `fail:<motivo>` ou `hang`. Os testes do driver de login o rodam num ConPTY de verdade; o app
  do sandbox também pode usá-lo (`ROUTER_CLAUDE_BIN`) para conferir o login sem conta real.
  `statusline-echo` faz de status line do usuário (o modo "meu comando" do router): lê o JSON
  do stdin até o EOF e imprime `eco: <modelo> encadeado=<ROUTER_STATUSLINE_CHAINED>`.
