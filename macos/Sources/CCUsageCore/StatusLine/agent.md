# StatusLine — a linha que a sessão de um grupo imprime

O que o `router statusline` escreve DEPOIS de gravar a amostra. Mora no núcleo, e
não na CLI, porque a prévia dos Ajustes vai desenhar com ESTE código: a prévia
nunca pode discordar do que a sessão mostra. O sensor (a gravação da amostra)
não passa por aqui.

Par do `windows/crates/router-core/src/statusline/`, portado dele.

## Arquivos
- `StatusLineView.swift` — o modelo (`Label`, `Window`, `Context`, `Style`) e o `render`: a linha inteira, pura.
- `StatusLinePaint.swift` — ANSI, cor por severidade, a barra, e a animação do `effort` (o brilho do `xhigh`, o arco-íris do `max`), com a fase entrando por parâmetro.
- `StatusLineFormat.swift` — texto: `resetWhen` (o "quando" de um reset), nome do modelo, tokens, caminho encurtado.
- `StatusLineSource.swift` — o que vem do ambiente: o branch pelo `.git/HEAD` e se o terminal aceita truecolor.
- `StatusLineSession.swift` — o JSON do Claude Code → `View`, e a sessão de EXEMPLO (`sample`) para a prévia e o `doctor`.

## Padrões
- **Puro.** Relógio (a fase), idioma, suporte a cor e calendário entram como `Style`. A mesma `View` com a mesma fase dá sempre a mesma linha — é o que torna a suíte determinística e a prévia reprodutível.
- **Nada de processo por render.** O branch vem de ler o `.git/HEAD`, não de rodar o `git`: a linha é desenhada a cada atualização da sessão.
- **O que não veio some, sem marcador.** O 1º render de uma sessão não traz `rate_limits`, e o `effort` só vem com modelo que o aceita. Espaço reservado para o que não existe é ruído permanente.

## Decisões com data
- **24/09/2026 — a linha completa virou o padrão.** Antes daqui saía `conta 5h 7d`. Como o router é dono da `statusLine` do perfil (a linha **é** o sensor) e a escreve por cima — inclusive no grupo padrão, que é o `~/.claude` do usuário —, quem tinha status line própria a perdia ao ativar a integração de terminal. A saída foi enriquecer a linha, não empobrecer a sessão: o GRUPO na frente e o e-mail da conta ativa no fim, que é onde a troca aparece. Descoberto no primeiro teste real do porte Windows.
- **24/09/2026 — o Terminal.app NÃO entra na lista de truecolor.** Ele faz 256 cores; a animação do `effort` sairia como lixo nele.
- **24/09/2026 — `resetWhen` é pública** para a janela de Grupos escrever o reset com ela quando essa tela ganhar os resets (PR seguinte). Duas superfícies escrevendo a mesma hora de dois jeitos é o tipo de divergência que faz o usuário desconfiar do número.

## Pendências
- A ESCOLHA do usuário (quais itens aparecem, ou rodar o comando dele) — é o próximo PR, par do `choice.rs` e do `command.rs` do Windows.
