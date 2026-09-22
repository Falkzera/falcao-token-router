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
- Demais suites herdadas do medidor (UsageStore, parsers, pricing).
- *(`PanoramaTests.swift` saiu em 18/09/2026 junto com o subsistema que testava.)*

## Padrões
- Fixtures `FakeKeychain`/`FakeAdapter` (topo de EngineTests). `seedLogin` simula login concluído.
- Todo bug real pego em uso vira teste de regressão nomeando o episódio (ex.: "Login expired de 26/ago").

## Pendências conhecidas
- Nenhuma.
