# falcao-token-router

App de menu bar para macOS que gerencia **grupos de contas do Claude Code** com rodízio automático: o usuário cria grupos (ex.: trabalho, pessoal), loga contas pelo fluxo oficial da Anthropic dentro do app, define ordem e limiar — e o motor troca a conta ativa sozinho quando o limiar bate, sem encerrar a sessão. Nasceu como fork do medidor de tokens `Ulpio/ClaudeTokenCounter` (MIT) e está virando produto pago.

**Nomes, e por que são três.** O produto é **Falcão Token Router** (exibição), `falcao-token-router` (repo e pasta), `FalcaoTokenRouter` (target SPM, `.app`, módulo), `com.synqo.falcao-token-router` (bundle id). O namespace de dados em disco é **`com.synqo.falcao-router`** e **fica como está**: a casa de cada conta é `<base>/accounts/<uuid>` e o item de chaveiro é `Claude Code-credentials-<sha256(caminho)[:8]>` — mudar a base muda o hash e deixa toda credencial inalcançável de uma vez. O nome comercial **não pode conter "Claude"/"Anthropic"**.

**Restrição que desenha tudo:** os termos da Anthropic reservam o token OAuth ao cliente oficial. O app **não faz nenhuma chamada de rede** — mede pelo sensor passivo (a status line entrega o `rate_limits` que o próprio Claude Code recebe) e autentica pedindo ao binário oficial (pty). O estudo de conformidade que sustenta isso está resumido no README (seção *The constraint that designs everything*).

## Stack

| Camada | Tecnologia |
|---|---|
| App | Swift 6 / SwiftUI, MenuBarExtra, macOS 26 (Tahoe) |
| Motor | `CCUsageCore` (SPM target puro, sem UI) |
| CLI | `router` (launch/statusline/is-group/rotate), embutido no .app |
| Testes | swift-testing (`@Test`/`@Suite`) — **via `./Scripts/test.sh`, nunca `swift test`** (ver CONTRIBUTING.md) |
| Build | SPM + `Scripts/bundle.sh` (empacota .app, assina ad-hoc) |

## Comandos

```bash
./Scripts/test.sh                     # roda os testes (swift test NÃO funciona aqui)
./Scripts/check-strings.sh            # chaves de localização × catálogos
./Scripts/bundle.sh --native --install  # builda o .app e instala em /Applications
```

## Estrutura

- `Sources/CCUsageCore/` — motor: engine de grupos/rotação, medição, store. Sem UI.
- `Sources/FalcaoTokenRouter/` — o app SwiftUI (painel, grupos, ajustes).
- `Sources/router/` — a CLI `router` embutida no bundle.
- `Tests/CCUsageCoreTests/` — testes do motor e do store.
- `Resources/{pt-BR,en}.lproj/` — catálogos de strings (checados por script).
- `docs/` — `ARCHITECTURE.md` (o mapa, em inglês, para quem chega de fora) e `PORTING.md` (o que uma porta Linux/Windows precisa trocar).

## 🚨 Este repositório é público

Nunca versionar aqui: e-mail de conta real, nome de empregador ou cliente,
consumo medido de conta compartilhada, screenshot com conta de verdade, ou
documento operacional pessoal. Em teste e exemplo: `conta1@exemplo.com`,
`/Users/exemplo`, organização `Acme`.

Não é hipótese. A história anterior teve de ser recomeçada porque commits,
testes e documentação citavam contas de trabalho de terceiros — e-mails de
colegas e quanto cada um consumia. Nada disso é necessário para usar nem para
contribuir.

## Regras de código

- Comentários e strings de UI em **pt-BR**; identificadores em inglês.
- Toda string de UI vem do catálogo com prefixo `panel|settings|alerts|format|groups` — `check-strings.sh` bloqueia órfãs, faltantes e literais soltos (`Text(verbatim:)` é a saída para o que não se traduz).
- Estado que a UI precisa ver ao vivo mora em propriedade **observável** do store — propriedade computada que lê disco não re-renderiza (bug real do botão "Ativar").
- **Nunca escrever `@State`.** Do SDK do macOS 26 em diante ele é macro do SwiftUI e o plugin `SwiftUIMacros` só vem no Xcode — sob Command Line Tools o alvo do app não compila (60 erros, 18/09/2026). Usar **`@ViewState`** (`Sources/FalcaoTokenRouter/ViewState.swift`), typealias de `SwiftUICore.State`, que é a property wrapper real e está no SDK.
- Trocar `CFBundleIdentifier` **perde as preferências**: `UserDefaults.standard` é indexado por ele. Há migração do domínio antigo em `AppSettings.migrateLegacyDefaults` — se o id mudar de novo, acrescente o anterior lá.
- Nada de chamada de rede no app.
- Datas/decisões do domínio (credencial, chaveiro, medição) têm comentário explicando o PORQUÊ — quase tudo aqui foi descoberto por observação e custa caro redescobrir.

## Documentação por pasta (agent.md)

Toda pasta com código tem um `agent.md`: propósito da pasta, o que cada arquivo faz em uma linha, padrões locais, decisões recentes com data, e pendências conhecidas.

Ao entrar numa pasta pra trabalhar, leia o `agent.md`. Ao sair com mudança significativa — arquivo novo, decisão arquitetural, pendência resolvida ou criada — atualize-o. Mudança trivial (typo, formatação) não pede atualização.

## Roadmap curto

- v1 (branch `feat/router-v1`, PR #1): grupos + rodízio + sensor + integração de terminal — **validada em uso real**.
- Pré-venda: ícone definitivo, licença Ed25519 offline, Sparkle, notarização (Developer ID). Nome resolvido em 18/09/2026; Ajustes+Grupos já unificados em abas.
- ~~Lacuna: o limite POR MODELO não chega no `rate_limits`~~ — **fechada em 18/09/2026** pela sonda ativa (`router measure` / botão "Medir contas"), que pergunta ao binário oficial. Ela também mede conta OCIOSA, que o sensor passivo nunca enxerga.

## Contribuindo

Conventional Commits, uma branch por assunto saindo da `main`, PR com squash.
Comentário explica o PORQUÊ; o QUÊ o código já diz.

O que trava um PR: `./Scripts/test.sh` vermelho, `check-strings.sh` reclamando,
ou `@State` reintroduzido (ver acima). Ver `CONTRIBUTING.md`.
