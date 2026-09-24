# Panel — agent.md

## Propósito
O painel do menu bar: a tabela de contas por grupo, o detalhamento da conta selecionada (ou da sessão medida), o valor equivalente em API e o rodapé.

## Arquivos
- `UsagePanel.swift` — a composição. `selectedAccount` mora aqui e desce como `Binding`: com seleção o cartão de baixo fala da CONTA, sem seleção fala da sessão do perfil medido.
- `AccountsSection.swift` — a tabela de contas. Define também `AccountHelp` (o tooltip da linha, compartilhado com Grupos), `ModelBadge`, `SessionsBadge` e `UsageAge` (limiares de 1h e 12h).
- `WindowReading.swift` — uma janela do `rate_limits` numa linha: rótulo e valor. As DUAS aparecem sempre; a que manda se distingue por peso e cor, não por ser a única visível.
- `UsageColor.swift` — o semáforo de consumo, definido uma vez. Painel e barra leem os mesmos limiares daqui: com limiares separados, um diria "tranquilo" enquanto o outro já alertava.
- `Formatters.swift` — formatação de duração, percentual e valor.

## Padrões
- **Número sem procedência não vai para a tela.** Toda medida carrega de qual janela veio, quem a mediu (sensor × sonda) e há quanto tempo.
- **Ausência não é zero.** Janela sem medida válida deixa a trilha vazia; conta sem amostra mostra "pronta", não "—".
- A linha inteira é alvo de clique (`contentShape`) e o tooltip é da linha, não do número: mirar 34 pontos com o mouse é pedir precisão que ninguém tem.
- Larguras de coluna vivem em `AccountsLayout`, usadas pelo cabeçalho, pelas linhas e pelo bloco "pronta" — senão nada alinha de conta para conta.

## Pendências conhecidas
- O detalhamento por conta não tem tokens/min nem valor: esses vêm do JSONL do perfil local e não existem para as outras contas. Preenchê-los atribuiria consumo alheio.
