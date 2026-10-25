# Alerts — agent.md

## Propósito
Decidir **o que** notificar. A entrega fica no alvo de UI (`FalcaoTokenRouter/Alerts`), que é quem fala com o `UNUserNotificationCenter`.

## Arquivos
- `Alert.swift` — o alerta como **fato**, sem texto: qual janela, qual percentual. Nenhuma frase aqui.
- `AlertPolicy.swift` — a decisão. Pura e determinística: mesma sequência de snapshots, mesma saída. Não lê relógio.
- `AlertPreferences.swift` — o que o usuário quer ser avisado. É um tipo, e não cinco booleanos numa assinatura, porque parâmetros do mesmo tipo trocados de lugar compilam e mudam o comportamento em silêncio.

## Padrões
- **O core não escreve frase.** Copy em português aqui obrigaria a localizar o core junto com o app, e faria os testes afirmarem sobre redação em vez de sobre comportamento.
- A política não tem relógio próprio. É o que torna rearme, anti-repetição e mudança de preferência no meio de uma janela testáveis.

## Decisões recentes
- 2026-10-04: "janela resetou" só quando o reset da janela anterior já passou. O `resetsAt` também muda quando a conta muda (o grupo padrão rodou de A para B), e o aviso "capacidade cheia de novo" saía com B a 60%. Mudou antes da hora: outra conta — a linha de base é refeita calada.

## Pendências conhecidas
- Nenhuma.
