# Pricing — agent.md

## Propósito
Quanto custa cada modelo, por milhão de tokens — para o medidor dizer o equivalente em API do que foi consumido.

## Arquivos
- `PricingTable.swift` — `Rates` (preço por milhão) e a tabela, **por modelo e por data**: preço muda com o tempo, e recalcular o passado com o preço de hoje daria um número que nunca existiu.

## Padrões
- **Modelo sem preço conhecido não vira `$0,00`.** Ele entra em `unknownModels` e o total é marcado como parcial (`Money.isPartial`). Zero é uma afirmação; ausência não é.

## Decisões recentes
- 2026-10-04: a tabela foi conferida com a pública de 25/09/2026. Entraram Opus 5.5 ($4/$20, fast $8/$40), Sonnet 5.5 ($2/$10) e Fable/Mythos 5.1 ($10/$50); a leitura de cache deixou de ser 0,1× para todos (Opus 5.5: 0,05×; Fable 5.1: 0,025×). O Sonnet 5 fica em $2/$10 — a volta prevista a $3/$15 em 01/09 não aparece na tabela oficial, e o total saía 50% acima. A tabela resolve de novo o modelo do evento (`ModelID.resolved`): o cache guarda `.unknown` para o que não se conhecia na hora da leitura.

## Pendências conhecidas
- A tabela é mantida à mão. Modelo novo aparece como desconhecido até alguém acrescentá-lo — que é o comportamento desejado, mas exige atenção a cada lançamento.
