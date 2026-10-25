# router_config_store — agent.md

## Propósito
O `RouterConfigStore` (≙ `macos/Sources/CCUsageCore/Engine/RouterConfigStore.swift`): o dono
do `config.json` e a ponte entre a UI e o motor. Um tipo só, repartido em arquivos por motivo
de mudança — era um arquivo de 869 linhas de produção até a régua de 600 (24/09/2026).

## Arquivos
- `mod.rs` — o tipo, a carga e a gravação do `config.json`, o quadro de uso e as sessões. Um
  `config.json` ilegível nunca é sobrescrito: vai para o lado no primeiro `save`.
- `accounts.rs` — contas: reservar casa, concluir login e relogin, descartar casa pendente,
  remover (só apaga pasta que o router criou), apelidar.
- `groups.rs` — grupos: criar, renomear, padrão, limiar, ordem (reordenar não derruba conta
  esquecida), apagar.
- `rotation.rs` — o que toca credencial: ativar, rotacionar, espelhar, e a medição em três
  passos que o app usa (`measure_plan` → `MeasurePlan::run` → `finish_measure`).
- `terminal.rs` — a integração com o terminal: os scripts de shell e a status line.

## Padrões
- Toda ação que toca credencial passa pelo `RotationEngine`, sob a trava entre processos
  (`engine_lock`).
- Erros são fatos tipados (`StoreError`); o texto fica com a UI.

## Pendências conhecidas
- O plano da medição decide o perfil de cada conta antes de sondar, e nada impede a rotação
  ou o "Usar" de ativar a conta no meio — aí a sonda roda na casa de uma conta ativa.
