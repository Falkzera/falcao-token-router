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
  (o app troca esse formato com a tela).
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
