# macos/ — o app macOS do Falcão Token Router

O app de menu bar em **Swift 6 / SwiftUI**, par do `windows/`. Nenhuma das duas
pastas é a raiz do projeto: a raiz guarda o que é do produto (README,
CONTRIBUTING, `docs/`, os workflows), e cada sistema tem a sua.

Esta pasta é autocontida: `Package.swift`, `VERSION`, `CHANGELOG.md` e
`Scripts/` são todos dela, e os scripts se localizam sozinhos
(`ROOT=".../$(dirname $0)/.."`), então rodam de qualquer lugar.

## Estrutura
- `README.md` — o guia de quem instala num Mac: DMG, quarentena, integração de terminal, compilar do fonte, e o rito de release.
- `CHANGELOG.md` — o histórico desta plataforma. O Windows tem o dele.
- `VERSION` — a única fonte da versão do macOS; a tag `macos-v$(VERSION)` tem de bater, e o `release.yml` confere.
- `Package.swift` — três alvos: `CCUsageCore` (motor, sem UI), `FalcaoTokenRouter` (app), `router` (CLI embutida).
- `Sources/CCUsageCore/` — motor: grupos, rotação, espelhamento de credencial, sensor, sonda, medidor. **Não importa SwiftUI** — é o que deixa tudo testável sem janela.
- `Sources/FalcaoTokenRouter/` — o app e **todas** as strings de usuário.
- `Sources/router/` — a CLI que o bundle carrega (`statusline`, `launch`, `is-group`, `rotate`, `measure`, `doctor`).
- `Tests/CCUsageCoreTests/` — a suíte do motor e do store.
- `Resources/{en,pt-BR}.lproj/` — os catálogos, conferidos pelo `check-strings.sh`.
- `Scripts/` — `test.sh`, `check-strings.sh`, `bundle.sh`, `dmg.sh`, `icon.sh`, `release.sh`.

## O que não é óbvio
- **`swift test` puro não funciona** sob Command Line Tools. Use `./Scripts/test.sh`, que injeta o plugin de macro e o `Testing.framework`. Está no `CONTRIBUTING.md`.
- **Nunca escrever `@State`** — do SDK do macOS 26 em diante é macro do SwiftUI e o plugin só vem no Xcode. Use `@ViewState` (`Sources/FalcaoTokenRouter/ViewState.swift`).
- **O `icon.sh` escreve fora desta pasta.** O banner e o card social são do produto, não da plataforma, e vão para `../docs/art` — ele usa `$REPO`, não `$ROOT`.
- **O que esta pasta compartilha com `windows/` não é código, é o formato em disco:** `config.json`, as casas por conta e `usage/<email>.json`. Mudar esse formato é mudar as duas plataformas.

## Decisões com data
- **24/09/2026** — a pasta nasce. Antes, este projeto era a raiz do repositório e o `windows/` era hóspede; a estrutura contradizia a documentação, que trata portes como cidadãos de primeira classe. `CHANGELOG.md` e `VERSION` vieram junto, porque o Windows já tinha os dele.
