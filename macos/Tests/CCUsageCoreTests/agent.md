# CCUsageCoreTests — agent.md

## Propósito
Testes do motor e do store — chaveiro/adapter falsos em memória, caminhos temporários; nada toca o sistema real. Rodar com `./Scripts/test.sh` (**`swift test` não funciona** — ver CONTRIBUTING.md).

## Arquivos
- `EngineTests.swift` — ConfigDir/hash de chaveiro (conferido contra itens reais), RotationEngine (ativar, espelhar, recusar duplicata, reativar preserva token vivo), decisão de rotação (presunção de fresca, fail-safe), GroupUsageReader (decaimento por reset, procedência da janela), `UsagePercent` (uma formatação só), AnthropicAdapter em arquivo real.
- `StoreTests.swift` — a API que a UI chama: grupos padrão/dedicado, login pendente (added/duplicate), relogin (renewed/wrongAccount/push pro grupo), remoção com órfãs, persistência.
- `LauncherTests.swift` — SessionLauncher e ShellIntegration (função de shell, append idempotente no profile, obsolescência da status line) e `ProfileIntegrityTests` (o `~/.zshrc` não-UTF-8 que era destruído).
- `ProbeTests.swift` — a sonda: parse da saída REAL do `claude /usage` (fixture capturada em 18/09/2026), virada de ano no reset, `probeConfigDir` (ativa vai pelo grupo), o por modelo entrando na decisão de rotação, e a preservação do bloco por modelo quando o sensor escreve por cima.
- `ProbeTests.swift` (suite `UsageOriginTests`) — procedência: amostra antiga sem a chave decodifica como sensor, a origem sobrevive ao disco, sensor por cima da sonda troca a origem mas preserva o modelo, e a origem chega ao `AccountUsage` que a UI lê.
- `SessionRegistryTests.swift` — o registro de sessões: leitura de um registro real, status desconhecido virando `.other` (e não `idle`), processo morto descartado, pid reciclado recusado pelo instante de início, e o `ctime` com dia de um dígito.
- `UsageHistoryTests.swift` — o histórico (21/10/2026): ordem, poda de 90 dias, linha estragada pulada, formato (sem fração, janela ausente sem chave, `models` omitido), **uma linha gravada pelo Windows lida aqui**, apagar só os `.jsonl`; e o `ProbeRecord` (amostra + ponto, histórico que falha não desfaz a amostra, sessão ausente não vira zero).
- `MeasureScheduleTests.swift` — a regra da agenda: nunca medida vence, 5 horas exatas vencem, a tentativa que falhou segura a conta.
- `HistoryStoreTests.swift` — o store com o histórico: o plano só com as vencidas, grupo a grupo; conta em dois grupos medida uma vez; a tentativa recente; a ativa pelo perfil do grupo; login aberto segura a agenda; apagar.
- `UsageHistoryChartTests.swift` — as séries dos gráficos, num fuso fixo (UTC−3): períodos de 7 e 28 dias, buraco de mais de 7h30, ausência fora do gráfico, marcas e nomes dos dias.
- Demais suites herdadas do medidor (UsageStore, parsers, pricing).
- *(`PanoramaTests.swift` saiu em 18/09/2026 junto com o subsistema que testava.)*

## Padrões
- Fixtures `FakeKeychain`/`FakeAdapter` (topo de EngineTests). `seedLogin` simula login concluído.
- Todo bug real pego em uso vira teste de regressão nomeando o episódio (ex.: "Login expired de 26/ago").

## Pendências conhecidas
- Nenhuma.
