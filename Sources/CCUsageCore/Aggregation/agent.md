# Aggregation — agent.md

## Propósito
Transformar a lista de eventos em **janelas e totais**: o bloco de 5 horas, os recortes de período, e os denominadores que o medidor usa quando não há número oficial.

Tudo aqui é **estimativa**, e é por isso que mora separado: a Anthropic não persiste localmente os horários de reset nem publica os limites do plano. Quem consome precisa apresentar como estimativa — ver `UsageSnapshot.Provenance`.

## Arquivos
- `BlockBuilder.swift` — agrupa eventos em janelas de 5h (`UsageBlock`). Derivado da atividade, não do servidor.
- `CeilingCalibrator.swift` — o denominador dos percentuais: o maior consumo já observado, com piso. Também `Pace`, a comparação com o ritmo típico do usuário.
- `PeriodAggregator.swift` — recortes de calendário (hoje/semana/mês) e a janela rolling de 7 dias.

## Padrões
- **Nunca saturar silenciosamente.** `Pace` substituiu um gauge de "% do teto semanal" porque aquele denominador só funcionava com uso estável: em fase de crescimento a barra vivia estourada, e indicador sempre no vermelho deixa de ser sinal.
- "Semana" tem DUAS definições, de propósito: a de calendário alimenta o **valor**, a rolling de 7 dias alimenta o **risco**. A UI rotula as duas.

## Pendências conhecidas
- O teto calibrado é sensível a um pico isolado: um bloco atípico levanta o denominador por muito tempo.
