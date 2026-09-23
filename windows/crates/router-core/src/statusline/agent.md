# router-core/src/statusline — a status line dos grupos

O que o `router statusline` imprime depois de gravar a amostra. Mora no núcleo porque a CLI
e o app (a prévia na aba Ajustes) desenham com o MESMO código: a prévia nunca discorda da
sessão. O sensor (a amostra) não passa por aqui.

## Arquivos
- `mod.rs` — os módulos.
- `choice.rs` — a ESCOLHA do usuário (`StatusLineChoice`), em `<base>\statusline.json`: modo `app`
  (a linha completa menos os itens TIRADOS — `hidden`, na ordem da linha) ou `command` (o
  comando do usuário depois do sensor; em branco, vale a linha do app). Leitura TOLERANTE:
  ausente, ilegível ou de outro formato = a de fábrica; item e modo desconhecidos são
  ignorados sem levar o resto. Gravação atômica. O `serde` passa pelo mesmo caminho tolerante
  (o app troca esse formato com a tela). `apply` tira da `View` o que foi escondido (o
  "horário do reset" vale para as duas janelas; tudo de fora = linha vazia, o "nada").
- `command.rs` — o modo "meu comando": `Shell` (Git Bash; sem ele, PowerShell — `detect`) e
  `process` (o processo como o Claude Code o monta), `bash_line` (1º termo `.sh` → `bash …`),
  `run` → `Outcome` (`Printed` só com código 0 e saída visível; `Failed` com código e começo do
  stderr; `NotStarted`; `TimedOut`). JSON + `\n` no stdin e EOF; sem janela; a árvore no
  `platform::job` (morre no prazo de 5 s, e quando o router sai); um filho em segundo plano
  que segura o stdout ganha 150 ms e vai junto. `CHAINED_ENV` marca o comando: um router
  dentro dele não roda o comando de novo.
- `session.rs` — o JSON do Claude Code → `View` (`view_from`, `label_for`, `window`, que o
  sensor também usa) e a sessão de EXEMPLO (`sample`: a prévia e o "Testar" dos Ajustes, o
  `doctor`), com os resets no futuro dentro das janelas.
- `view.rs` — a LINHA, pura: `● grupo │ Modelo effort │ branch │ contexto │ 5h ↻ 7d ↻ │ $custo │
  e-mail`. Cor por grupo (posição na lista), Fable em vermelho, as cores do seletor do `/effort`
  (brilho no `xhigh`, arco-íris no `max`, pela fase do relógio), cinza explícito com truecolor,
  dias do reset no idioma do Windows, branch pelo `.git/HEAD` (com teto para os testes), caminho
  encurtado a partir da home. O que não veio no JSON fica de fora, sem marcador.

## Padrões
- Puro: relógio (fase), idioma e suporte a cor entram como `Style`; o que a linha mostra entra
  como `View`, já montada (a CLI a monta a partir do JSON do Claude Code).
- Nada de processo por render no modo padrão: o branch vem do `.git/HEAD`, não do `git`.

## Decisões
- 23/09/2026: a linha completa virou o padrão do app (antes era `conta 5h 7d`), no 1º teste real.
- 23/09/2026: saiu da CLI para o núcleo (era `router-cli/src/statusline_view.rs`) para a prévia
  dos Ajustes usar o mesmo código.
- 23/09/2026 (status line configurável, decidido com o usuário): a escolha mora num arquivo à
  parte na base — a CLI não lê os ajustes do app (Roaming) e o `config.json` é o formato
  combinado com o macOS. O modo "não usar a linha do app" roda o comando DO USUÁRIO (a linha
  mínima e o "nada" já saem desligando itens).
- 23/09/2026 (lido no JS do `claude.exe` 2.1.280): a status line passa pelo executor dos
  hooks — Git Bash por `spawn(comando, {shell: bash})` (= `bash -c`) com a pasta do bash na
  frente do `PATH` e o prefixo `bash ` para um `.sh`; sem Git Bash, `pwsh`/`powershell` com
  `-NoProfile -NonInteractive -ExecutionPolicy Bypass -Command`; `windowsHide`; o JSON + `\n` e
  o `end()` do stdin (o spike nunca viu o EOF chegar ao sensor — um pipe herdado por outro
  processo fica aberto; o router fecha o do comando de fato, com teste); a saída só com código
  0, cada linha aparada, vazias fora; prazo dos hooks
  (10 min) e cancelamento a cada atualização nova. O router copia tudo, menos o prazo (5 s) e
  o destino da árvore (morre junto). O `CLAUDE_CODE_SHELL_PREFIX` não é reaplicado ao comando.
- 23/09/2026: o `view_from` (JSON → `View`) saiu da CLI para cá: a prévia monta a linha pelo
  mesmo caminho da sessão, só que com a sessão de exemplo.
