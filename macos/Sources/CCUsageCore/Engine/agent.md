# Engine — agent.md

## Propósito
O motor do Falcão Router: contas, grupos, troca de credencial, medição passiva e o store que a UI observa. Sem UI, sem rede — tudo aqui lê/escreve disco e chaveiro locais.

## Arquivos
- `RouterConfigStore.swift` — o store observável que a UI chama: CRUD de grupos/contas, login/relogin pendente, rotação, integração de shell. `config` persiste em `config.json`.
- `RotationEngine.swift` — a troca em si: ativar conta (casa→grupo), espelhar vivo→casa, relogin (casa→grupo via `pushHomeToGroup`), decidir rotação, e `probeConfigDir` (por qual perfil sondar cada conta). As regras anti-morte-de-conta moram aqui.
- `GroupUsageReader.swift` — amostras → uso por conta. Considera TRÊS janelas: 5h, 7d e a mais apertada POR MODELO (esta vem da sonda, `Usage/ClaudeUsageProbe.swift`). Janela com reset vencido decai; conta sem amostra fica FORA do mapa (presumida fresca pelo motor). `detailByAccount` devolve `AccountUsage` (fração + de QUAL janela veio + as duas medidas); `usageByAccount` é a mesma coisa só com a fração, para a rotação.
- `GroupUsage.swift` — `GroupUsageSample` + store de amostras por e-mail, e `UsageOrigin` (sensor × sonda). A origem é serializada como opcional: amostra gravada antes de a sonda existir só podia vir do sensor.
- `AnthropicAdapter.swift` — o contrato com o Claude Code: nome do item de chaveiro (hash do caminho), `.claude.json` (identidade + `hasCompletedOnboarding`).
- `SessionLauncher.swift` — decide a conta e monta o plano de lançamento do `router launch`.
- `AccountLoginService.swift` — lê o desfecho do login oficial no disco (quem roda o login é a `LoginSession`, na camada de app).
- `ProfileSharing.swift` — symlinks de `projects/`, `history.jsonl`, `skills/` etc. do `~/.claude` para perfis de grupo (`--resume` compartilhado).
- `ShellIntegration.swift` — a função `claude()` de shell, a status line nos settings.json (`installStatusLine` / `statusLineIsStale`) e o append idempotente no `~/.zshrc`.
- `SessionRegistry.swift` — quais sessões do Claude Code estão vivas **em cada perfil** (`<perfil>/sessions/<pid>.json`, escrito por ele). Traz `ProcessLiveness`, que confere pid + instante de início para não ressuscitar sessão morta com pid reciclado.
- `KeychainStore.swift` — protocolo (`read`/`write`/`exists`/`delete`) + `SecurityCLIKeychain` (via `security`).
- `RouterPaths.swift` — onde tudo mora em Application Support.
- `ConfigDir.swift` / `AccountModel.swift` / `GroupModel.swift` / `Provider.swift` / `ProviderEnv.swift` — tipos do domínio.

## Padrões
- Estado que a UI observa é propriedade armazenada `@Observable` no store — computado que lê disco não re-renderiza.
- Toda regra de credencial tem comentário com o PORQUÊ (descobertas de agosto/2026; ver skill `falcao-router`).
- `now: Date = Date()` como parâmetro onde o tempo decide, para teste.

## Decisões recentes
- 2026-08-26: reativar a conta já ativa NUNCA copia casa→grupo (o item do grupo é a cópia viva; refresh token gira). O movimento é o inverso, e o espelhamento periódico roda no `rotateAll`.
- 2026-08-26: conta sem amostra é presumida fresca e é escolhível (o contrário criava deadlock: só mede quem serve).
- 2026-08-26: perfil dedicado nasce com `hasCompletedOnboarding: true`, senão o Claude Code abre o assistente de login ignorando o chaveiro.
- 2026-08-26: relogin reusa a casa da conta; se ela está ativa, `pushHomeToGroup` leva a credencial nova ao item do grupo (único caso legítimo de casa→grupo com conta ativa).
- 2026-08-28: o uso passou a carregar PROCEDÊNCIA (`AccountUsage.window`). O painel mostrava "66%" sem dizer que era o semanal, ao lado de uma status line escrita `5h 1%` — parecia contradição e custou a confiança no número. No empate entre as janelas vale a de 7 dias, que leva dias para aliviar.
- 2026-08-28: `store.usageDetail` é a propriedade observável que a UI consome; `usageSnapshot` continua existindo para o motor de rotação.

- 2026-09-18: o store ganhou `integrationIsStale` / `healShellIntegration`, e o app os chama na subida. O `shell.sh` e a `statusLine` guardam o caminho ABSOLUTO do `router`, que mora dentro do `.app`; renomear ou mover o app deixava os dois apontando para um caminho morto, e o sintoma era silencioso — `claude <grupo>` caía no `command claude` final e abria no `~/.claude`, na conta errada. Cura automática, e não botão, porque quem é atingido é quem não sabe que precisa apertá-lo.
- 2026-09-18: `ensureInProfile` passou a trabalhar em BYTES e a acrescentar de verdade (`FileHandle.seekToEnd`). Antes lia o `~/.zshrc` como `String` UTF-8 com `try? ... ?? ""` e reescrevia `existing + block`: um profile que não decodificasse em UTF-8 (um alias com acento em latin-1 basta) virava string vazia e o arquivo inteiro do usuário era substituído pelo bloco. Perda de dados, silenciosa, sem backup. Teste de regressão em `ProfileIntegrityTests`.
- 2026-09-18: `ProviderEnv` ganhou `credentialKeys`. Só proxy não bastava: `ANTHROPIC_API_KEY`/`AUTH_TOKEN` fazem a sessão ser servida por chave de API em vez da conta OAuth do grupo — e o sensor ainda grava a amostra sob o e-mail do perfil, carimbando na conta um consumo que não é dela. `ANTHROPIC_BASE_URL` e as variantes Bedrock/Vertex são o `teamclaude` com outro nome.

- 2026-09-18: o limite POR MODELO entrou na decisão. `GroupUsageSample.models` guarda as janelas da sonda com **carimbo de tempo próprio**, e `GroupUsageStore.write` as PRESERVA quando a amostra nova não as traz — sem essa costura, a primeira mensagem depois de uma sondagem apagaria o número do Fable, que é o único que enxerga o limite que estoura primeiro.
- 2026-09-18: `probeConfigDir` — conta ativa é sondada pelo perfil do GRUPO, **nunca** pela casa. Sondar a casa de uma conta ativa faria o `claude` renovar com o refresh token velho; se ele ainda valer, a renovação gira a cadeia e invalida a cópia do grupo — a sessão viva do usuário cai em "Login expired" no meio do trabalho. É o episódio de 26/ago com outro gatilho.

- 2026-09-18: entra o `SessionRegistry`. O registro de sessões do Claude Code é **por perfil** — sete sessões em `~/.claude/sessions/` e uma em `<grupo>/sessions/`, conferido no disco. É a resposta que o `/status` não dá ("qual sessão roda em qual grupo, e por qual conta") e onde o modo de falha silencioso aparece: sessão que devia estar num grupo e subiu no perfil padrão fica contada do lado errado.
- 2026-09-18: `removeAccount` passou a **apagar a credencial e a casa** da conta. Antes só o registro saía, e o refresh token ficava vivo no chaveiro do usuário para sempre — num produto pago, promessa quebrada. O item do GRUPO não é tocado mesmo quando é esta conta que o serve: pode haver sessão viva atendida por ele, e derrubar o trabalho de alguém não é consequência aceitável de arrumar uma lista.
- 2026-09-18: `homeKeychainService(for:)` no motor. O store chamava `AnthropicAdapter()` direto para descobrir o item a apagar, o que divergiria do nome que o motor usa em qualquer outro provedor — apagando nada, em silêncio. Pego por teste com `FakeAdapter`.
- 2026-09-18: `config.json` passou a 0600. Não há segredo ali (a credencial mora no chaveiro), mas há e-mail, organização e plano de cada conta, e o `umask` padrão deixava isso legível para qualquer usuário da máquina.

- 2026-09-19: a medida da conta passou a carregar **procedência** (`UsageOrigin`). O medidor já fazia isso (`UsageSnapshot.Provenance`: live/cached/derived); o roteador não, e depois da sonda a tela atribuía ao sensor um número que a sonda tinha buscado — com a frase "medido pela sessão da própria conta", falsa justamente nas contas ociosas, que sessão nenhuma serviu. Uma amostra pode carregar as DUAS procedências: 5h/7d do sensor e as por modelo da sonda, cada uma com seu carimbo.

- 2026-09-23: `ProviderEnv.credentialKeys` ganhou oito nomes que o porte Windows (issue #5, @viniventur) achou no JavaScript do Claude Code e que eu confirmei com `strings` no binário do macOS. O grave é `CLAUDE_CODE_OAUTH_TOKEN`: exportada, a sessão é servida por esse token e não pela conta do grupo — o `router launch` deixava passar. `CLAUDE_SECURESTORAGE_CONFIG_DIR` tem precedência sobre `CLAUDE_CONFIG_DIR` para achar a credencial: setada, o app escreveria o item do grupo enquanto o Claude Code lê outro.

## Pendências conhecidas
- `removeAccount`/`removeGroup` não apagam o item de chaveiro nem a pasta `accounts/<uuid>`: a credencial da conta removida continua viva no chaveiro do usuário. Deliberado por ora (ver comentário em `removeGroup`), mas num produto pago "remover a conta" que deixa o refresh token para trás é promessa quebrada.
- `wrongAccount` no relogin deixa a credencial do intruso na casa até a próxima tentativa (inofensivo; a identidade do registro não muda).
