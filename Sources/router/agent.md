# router (CLI) — agent.md

## Propósito
A CLI embutida no .app (`Contents/MacOS/router`) que o shell e a status line chamam. Fina de propósito: decide via `CCUsageCore` e executa.

## Arquivos
- `main.swift` — os quatro comandos:
  - `statusline` — o SENSOR: lê `rate_limits` do stdin (o Claude Code entrega), grava a amostra por e-mail e imprime a linha colorida.
  - `launch <grupo> -- <args>` — escolhe/ativa a conta (`SessionLauncher`), garante status line + ProfileSharing no perfil, e `execvp` o claude com `CLAUDE_CONFIG_DIR` (ou sem, no grupo padrão).
  - `is-group <nome>` — a função de shell pergunta antes de rotear.
  - `rotate` — uma volta de espelhamento + rotação (para agente periódico externo; o app tem laço próprio).
  - `measure [grupo]` — a SONDA: mede as contas pelo binário oficial, inclusive o limite por modelo e o número das contas OCIOSAS (que o sensor nunca vê). Respeita `probeConfigDir`: conta ativa vai pelo perfil do grupo, nunca pela casa.
  - `doctor` — diagnóstico: config, `shell.sh` apontando para ESTE binário, `source` no `~/.zshrc`, sensor por perfil, conta ativa e idade da amostra, **as sessões vivas de cada grupo com o que cada uma está fazendo**, e conta ativa em dois grupos. Sai 1 se algo está torto.

## Padrões
- Nada de lógica de domínio aqui — ela mora em `CCUsageCore` para ser testável.
- Mensagens de erro em pt-BR no stderr; `→ Grupo: conta` no stderr antes do exec é o sinal visível de roteamento certo.

## Decisões recentes
- 2026-08-26: `rotate` também espelha (`mirrorActive`) antes de decidir, igual ao laço do app.
- 2026-08-28: o `%` da status line passou a vir de `UsagePercent` (CCUsageCore), a mesma função que o painel usa. Antes o sensor arredondava e o painel truncava: 0,666 saía 67% aqui e 66% lá, e a divergência de um ponto fazia o usuário duvidar da medição.

- 2026-09-18: entrou o `doctor`. Todos os modos de falha daqui são silenciosos — a função de shell apontando para um `.app` renomeado não dá erro, só abre na conta errada — e a seção "Diagnóstico" da skill era uma lista de comandos manuais que ninguém lembra na hora. Um comando que NOMEIA o problema vale mais que seis que exibem estado.

- 2026-09-18: entra o `measure`. O sensor passivo só mede quem está servindo, e só 5h/7d; a sonda cobre as duas lacunas. É comando e não laço porque cada conta custa um processo Node subindo do zero (~3s) e uma requisição de verdade.

## Pendências conhecidas
- Nenhuma.
