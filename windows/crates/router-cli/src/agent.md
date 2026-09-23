# router-cli — a CLI `router` (≙ Sources/router)

Binário `router.exe`. Sem argumento, o comando é `statusline` (igual ao macOS). Mensagens
em pt-BR, como as da CLI do macOS; `router: <msg>` no stderr e código 1 nas falhas; uso
desconhecido sai com 2.

## Arquivos
- `main.rs` — despacho por argv: `statusline`, `launch`, `is-group`, `rotate`, `doctor`, `measure`.
- `shared.rs` — base, home (`%USERPROFILE%`), `load_config`, credencial em arquivo COM a guarda
  do perfil padrão, `fail`, o shell detectado; reexporta o `run_with_timeout` do núcleo.
- `statusline.rs` — o **sensor**: lê `rate_limits` do stdin (thread + prazo de 250 ms, pega o 1º
  JSON sem esperar EOF), perfil = `CLAUDE_CONFIG_DIR` → `--profile` embutido → padrão; grava a
  amostra **só com e-mail e ao menos uma janela**, sob a trava (espera 100 ms); monta o que a
  linha mostra (`view_from`: o JSON do Claude Code + o grupo dono do perfil, `label_for`, pelo
  `config.json`), imprime e sai 0 sempre.
- `statusline_view.rs` — a LINHA, pura: `● grupo │ Modelo effort │ branch │ contexto │ 5h ↻ 7d ↻
  │ $custo │ e-mail`. Cor por grupo (posição na lista), Fable em vermelho, as cores do seletor
  do `/effort` (brilho no `xhigh`, arco-íris no `max`, pela fase do relógio), cinza explícito
  com truecolor, dias do reset no idioma do Windows, branch pelo `.git/HEAD` (com teto para os
  testes), caminho encurtado a partir da home.
- `launch.rs` — `launch` (aceita `<g> -- args` e `<g> args`; ativa sob a trava; planta o sensor;
  liga o compartilhamento; ambiente direto; sobe o `claude` como FILHO ignorando Ctrl+C no router
  e repassa o código de saída), `is-group` (mudo, só o código) e `rotate` (mudo).
- `measure.rs` — a sonda por conta (ativa pelo perfil do grupo), saída igual à do macOS.
- `doctor.rs` — as checagens do macOS + as do Windows (scripts, `$PROFILE` das duas edições e
  `.bashrc`, `.bash_profile`, política de execução sem o escopo Process, sensor de cada grupo
  rodando DE VERDADE pelo shell detectado, statusLine de projeto competindo, links quebrados,
  variáveis que desviam a sessão, `claude` e versão).

## Decisões
- 22/09/2026 (spike): stdin da status line **nunca fecha** → leitor com prazo; 1º render vem sem
  `rate_limits` → não grava amostra.
- 22/09/2026: sem `exec` no Windows — o `router` espera o `claude` e sai com o código dele. Ctrl+C
  com handler PRÓPRIO (o `SetConsoleCtrlHandler(NULL, TRUE)` seria herdado pelo `claude`).
- 22/09/2026: a função do PowerShell engole o `--` do `$args` (medido nas duas edições) → o
  `launch` aceita as duas formas.
- O `doctor` não escreve nada; a status line que ele roda recebe `{}` (sem janela = sem amostra).
- 22/09/2026 (fase 5): edições do PowerShell, política efetiva, `function claude` do usuário e o
  perfil de login do Git Bash foram para o núcleo (`engine::terminal_report`), que o app também
  usa; o `run_with_timeout` foi para `platform::process`. Mensagens do `doctor` iguais. De
  quebra, `function claude-gov` deixou de contar como função `claude` (o `\b` casava o `-`).

- 23/09/2026 (teste real, a pedido do usuário): a linha do grupo era `conta 5h 7d` e substituía
  uma status line completa do usuário, que sumia nas sessões dos grupos. Virou o layout padrão
  do app, com o GRUPO no lugar do nome do perfil e o e-mail da conta ativa no fim (é onde se vê
  a troca). Tudo do JSON documentado da status line; nada de processo por render (o branch vem
  do `.git/HEAD`). Sem `rate_limits` (1º render) a linha só omite as janelas — o "sem uso ainda"
  saiu. O sensor e a regra de cor das janelas (0,70/0,90) não mudaram.

## Pendências
- Ctrl+C durante `launch` só dá para conferir à mão (enviar Ctrl+C num teste atingiria o próprio
  runner): fica no roteiro ponta a ponta da Fase 7.
