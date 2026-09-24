# Models — agent.md

## Propósito
Os tipos do domínio do MEDIDOR: o que é um evento de uso, quanto ele vale, e o retrato que a UI desenha.

## Arquivos
- `UsageEvent.swift` — uma mensagem do assistente com uso de token, extraída de uma linha do JSONL.
- `ModelID.swift` — o modelo, agrupado por faixa de preço. Gerações que compartilham a mesma tabela colapsam num caso só: separá-las não mudaria nenhum número.
- `Money.swift` — valor em USD que **sabe quando está incompleto**. `isPartial` existe para que modelo sem preço conhecido nunca vire zero: a soma do que dá para precificar é preservada e o total é marcado como piso.
- `UsageSnapshot.swift` — tudo que a UI precisa desenhar, já calculado. Carrega `Provenance` (live / cache / derivado) por gauge, porque um número sem procedência é um número em que não se pode confiar.

## Padrões
- **Desconhecido nunca vira zero nem é descartado em silêncio.** Modelo sem preço entra em `unknownModels` e a UI mostra quais são; janela sem nome de modelo é descartada em vez de virar barra anônima.
- Gauge oficial não tem `tokens`/`ceiling`; gauge derivado não tem `severity`. Cada caminho expõe só o que de fato sabe.

## Pendências conhecidas
- Nenhuma.
