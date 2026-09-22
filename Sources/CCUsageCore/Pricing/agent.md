# Pricing — agent.md

## Propósito
Quanto custa cada modelo, por milhão de tokens — para o medidor dizer o equivalente em API do que foi consumido.

## Arquivos
- `PricingTable.swift` — `Rates` (preço por milhão) e a tabela, **por modelo e por data**: preço muda com o tempo, e recalcular o passado com o preço de hoje daria um número que nunca existiu.

## Padrões
- **Modelo sem preço conhecido não vira `$0,00`.** Ele entra em `unknownModels` e o total é marcado como parcial (`Money.isPartial`). Zero é uma afirmação; ausência não é.

## Pendências conhecidas
- A tabela é mantida à mão. Modelo novo aparece como desconhecido até alguém acrescentá-lo — que é o comportamento desejado, mas exige atenção a cada lançamento.
