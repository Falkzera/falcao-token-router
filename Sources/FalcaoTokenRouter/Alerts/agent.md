# Alerts (app) — agent.md

## Propósito
Entregar ao usuário o que `CCUsageCore/Alerts` decidiu. Mora aqui porque fala com o `UNUserNotificationCenter` — e porque é aqui que vivem as frases.

## Arquivos
- `AlertCoordinator.swift` — liga o store à política e a política à entrega. O `UsageStore` continua sem saber que notificação existe: publica o snapshot, e quem se interessa escuta.
- `UserNotificationPresenter.swift` — traduz `Alert` (que carrega fato, não frase) em notificação do sistema. `AlertPresenting` é protocolo para o coordenador ser exercitável sem disparar notificação de verdade.

## Padrões
- **Toda a redação vive aqui**, junto do resto das strings de usuário. É o que permite localizar o app sem localizar o core.
- A permissão é pedida no momento em que o usuário LIGA os alertas, nunca no lançamento: app sem janela que pede permissão ao subir pede sem contexto, e a negativa é permanente.
- Chave ligada sobre permissão negada é chave que mente — os Ajustes mostram o estado real do sistema.

## Pendências conhecidas
- Nenhuma.
