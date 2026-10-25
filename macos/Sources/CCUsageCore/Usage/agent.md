# Usage — agent.md

## Propósito
De onde vem o número de uso, e **com qual procedência**. Duas fontes convivem aqui, e a diferença entre elas é o assunto da pasta inteira:

- **Sensor passivo** (fora daqui, em `Engine/GroupUsage.swift` + `router statusline`): lê o `rate_limits` que o Claude Code entrega no stdin da status line. Roda a cada mensagem, de graça, e só enxerga **5h e 7 dias**.
- **Sonda ativa** (`ClaudeUsageProbe`, aqui): pergunta ao binário oficial via `claude --print /usage`. Roda sob demanda, custa um processo Node por conta, e é a **única** que enxerga o limite **por modelo** — e que mede **conta ociosa**, que nunca serviu mensagem nenhuma.

Nenhuma das duas fala com `api.anthropic.com`. Quem faz requisição é o cliente oficial, com a credencial dele.

## Arquivos
- `ClaudeUsageProbe.swift` — a sonda: acha as janelas em `claude /usage`, inclusive `Current week (Fable)`. Roda o processo (prazo, stdin nulo, stderr descartado, scratch fixo) e faz o parse. `output` é injetável, para o teste exercitar o texto sem login nem rede.
- `ProbeRecord.swift` — o que uma medição da sonda grava, num lugar só: a amostra (`usage/`, origem `probe`) e o ponto do histórico (`history/`). O botão, a agenda de 5 em 5 horas e o `router measure` passam por aqui; falha no histórico não desfaz a amostra.
- `ClaudeBinary.swift` — onde o `claude` está instalado. No core porque o app (login em pty) e a CLI (sonda) precisam dele, e `which` não resolve num app lançado pelo Finder.
- `UsagePercent.swift` — a conversão fração→% **num lugar só** (o sensor arredondava e o painel truncava: 0,666 saía 67% numa tela e 66% na outra).
- `UsageReport.swift` / `UsageReportDecoder.swift` — o payload de uso da Anthropic, decodificado a partir de `limits[]` (as chaves de topo são codinomes internos que giram a cada ciclo).
- `CachedUsageReader.swift` — `cachedUsageUtilization` de `~/.claude.json`, o fallback do MEDIDOR (não do rodízio).
- `UsageSourcePolicy.swift` — função pura que escolhe entre o número ao vivo (a amostra recente do sensor) e o cache, e devolve a procedência que a UI mostra: `live`, `cached(age:)` ou `derivedOnly`.

## Padrões
- **Falha nunca vira zero.** Toda leitura que não deu certo devolve `nil` e quem chama cai para a próxima fonte ou mostra ausência. Uma barra vazia lê como "livre", que é o oposto do que se sabe.
- **Todo número carrega idade e origem.** A sonda tem carimbo próprio (`ModelUsage.sampledAt`), separado do carimbo do sensor: dizer uma idade só para as duas seria afirmar que o Fable de ontem é tão fresco quanto o 5h de agora.
- `now: Date` entra por parâmetro onde o tempo decide, para o teste não depender do relógio.
- Endpoint e formato são indocumentados: o parser tolera o que não conhece e nunca lança por campo ausente.

## Decisões recentes
- 2026-10-04: sem amostra recente do sensor, o painel mostra o cache com a idade — não "credencial expirada · rode o Claude Code", que era o que aparecia depois de uma hora sem usar a conta do perfil padrão. Saíram o `LiveUsageError` e os estados `credentialExpired`/`liveUnavailable` (com o ícone de "sem conexão"): eram da época em que "ao vivo" era uma chamada de rede com token, e o sensor não tem credencial para expirar nem conexão para cair. Os textos de "busca ao vivo" viraram "número ao vivo".
- 2026-09-18: entra a **sonda ativa**. Fecha a lacuna conhecida da v1 — o limite por modelo não chega no `rate_limits`, e é ele que estoura primeiro (uma conta travou com `5h 91%` / `7d 79%` e `Fable 100%`). As bandeiras do comando não são cosméticas: `--print` evita o diálogo de confiança de diretório, `--no-session-persistence` evita gravar um transcript por sondagem, e `--strict-mcp-config` sem `--mcp-config` impede que cada medição suba os servidores MCP do usuário.
- 2026-09-18: a telemetria fica **ligada de propósito**. `DISABLE_TELEMETRY` e `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` também desligam a consulta de feature flags, e a linha semanal por modelo está atrás de um desses portões — com qualquer um setado, `/usage` para de imprimi-la. Descoberta do `codenotch` (MIT), confirmada aqui.
- 2026-09-18: a data de reset é parseada com **duas grafias** (`h:mma` e `ha`), porque os minutos somem na hora cheia: `Sep 18 at 7:29pm`, mas `Sep 21 at 9am`. Um padrão só perderia o reset em uma hora de cada sessenta.
- 2026-10-04: saíram `CredentialSource`, `KeychainCredentialSource` e `CachedCredentialSource`. Eram a leitura do item de chaveiro do Claude Code — com um modo capaz de extrair o `accessToken`, sobra da época em que o app buscava uso pela rede — e o único leitor que restava era o `PlanDetector`, que agora lê o plano do `.claude.json`. Nada no macOS decodifica mais a credencial.
- 2026-09-18: o ano do reset não é impresso e é **escolhido**: o candidato mais próximo de `now` entre ano passado, este e o que vem. Qualquer outra regra erra a virada do ano numa das direções.

- 2026-10-21: a sonda deixou de ser só "botão e comando": com o app aberto ela roda **de 5 em 5 horas por conta** (`Engine/MeasureSchedule.swift`), para a aba Histórico — umas 5 medições por conta por dia, com uma chave nos Ajustes que a desliga. Nunca no laço de 3 minutos. A montagem da amostra, que o store e a CLI faziam cada um à mão, foi para o `ProbeRecord`.
- 2026-10-25: saída de código zero sem nenhuma linha `Current` é "sem login" (`ClaudeUsageProbe.interpret`), a regra do Windows. O `claude` deslogado sai com 0 e só o resumo do `--print`; antes isso caía em `unrecognized`, e a conta nunca ganhava o selo de sem login. Formato novo é só quando há linha `Current` e ela não se lê.

## Pendências conhecidas
- A sonda mede uma conta por vez, em série. Cinco contas levam ~15s. Paralelizar é possível, mas cada uma sobe um Node — vale medir a pressão antes.
