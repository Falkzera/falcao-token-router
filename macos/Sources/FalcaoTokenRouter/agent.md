# FalcaoTokenRouter (app) — agent.md

## Propósito
O app SwiftUI de menu bar: painel de uso e UMA janela com duas abas — Grupos (o produto) e Medidor (os ajustes). Toda lógica de domínio vem de `CCUsageCore`; aqui é apresentação e orquestração.

## Arquivos
- `App.swift` — cenas (MenuBarExtra, Window `home`), `AppDelegate` (política de Dock + reabrir no clique do ícone), stores estáticos, laço de rotação de 180s (`refreshUsage` + `rotateAll`), e o `CommandGroup(replacing: .appSettings)` que devolve o ⌘,.
- `Settings/HomeWindow.swift` — a janela única: `TabView` de Grupos e Medidor. Define `HomeTab`, o store `HomeNavigation` (qual aba abrir — vem do painel, de outra cena), `SettingsCommand` (o item de menu do ⌘,) e `HomeWindowID` (o id da cena num lugar só).
- `Panel/UsagePanel.swift` — o painel do menu bar: contas por grupo, sessão (com rótulo do perfil medido) OU detalhamento da conta selecionada, valor, rodapé. `selectedAccount` mora aqui e desce como `Binding` para a tabela.
- `Panel/AccountsSection.swift` — a tabela de contas por grupo (rótulo da janela, barra, %, idade da amostra, "pronta"). Define `AccountHelp` (o tooltip da linha, compartilhado com Grupos), `ModelBadge` (a marca de que quem manda é uma janela por modelo), `SessionsBadge` (quantas sessões vivas o grupo tem) e `UsageAge` (limiares `stale` 1h / `veryStale` 12h), compartilhado com `GroupsView`.
- `Panel/WindowReading.swift` — rótulo + valor de UMA janela do `rate_limits`, usado pelo painel e pela tela de Grupos para as duas aparecerem lado a lado.
- `Settings/GroupsView.swift` — a aba do produto: cartões de grupo, login/relogin em folha (pty), integração de terminal, confirmações destrutivas.
- `Settings/SettingsView.swift` — a aba do MEDIDOR (plano, sensor, alertas, teto, sistema).
- `LoginSession.swift` — roda `claude auth login` num pty e observa a saída (link + "Login successful").
- `RouterBinary.swift` — caminho do `router` embutido no bundle.
- `ViewState.swift` — o typealias que substitui `@State` (ver Padrões).
- `MenuBarLabel.swift`, `GaugeMark.swift`, `Panel/Formatters.swift`, `Panel/UsageColor.swift` — apresentação.
- `Alerts/` — notificações de limiar/reset.
- `Settings/LoginItem.swift`, `Settings/SettingsFormState.swift` — abrir no login; estado do formulário.

## Padrões
- Toda string de UI no catálogo (`check-strings.sh` bloqueia o resto).
- Botão que muda estado dá feedback visível ≤2s (flash "✓", label que troca). Ação destrutiva SEMPRE confirma.
- **Nunca `@State`** — do SDK do macOS 26 em diante é macro do SwiftUI e o plugin `SwiftUIMacros` só vem no Xcode; sob Command Line Tools o alvo inteiro deixa de compilar. Usar `@ViewState` (`ViewState.swift`), typealias de `SwiftUICore.State`. Posse de store continua em `static let` no `App`.
- O app é `LSUIElement`: janelas precisam de `NSApp.activate()` ao abrir.
- **`NSApp` é nil durante `App.init()`.** É um global implicitamente desembrulhado que só existe depois que o `NSApplication` sobe — tocá-lo no init derruba o app no lançamento. Usar `NSApplication.shared`, que cria a instância se preciso.

## Decisões recentes
- 2026-08-28: **Ajustes e Grupos viraram UMA janela com abas.** A janela de Ajustes
  abria com uma seção — a primeira da tela — cujo conteúdo inteiro era um botão "Abrir
  Grupos", e a de Grupos tinha uma engrenagem de volta. Ponte de mão dupla entre duas
  telas é o sintoma de que são uma só: o usuário não escolhia entre elas, saltava. Some
  a seção-ponte, some a engrenagem, some o título redundante no topo de Grupos (a aba já
  o diz). O rodapé do painel mantém os DOIS rótulos, "Grupos" e "Ajustes", porque são o
  que o usuário procura — mudou o destino, não o vocabulário.
- 2026-08-28: a cena `Settings` foi removida, e com ela o `SettingsLink` do painel (sem
  a cena ele não teria o que abrir). O ⌘, e o item "Ajustes…", que a cena dava de graça,
  voltam por `CommandGroup(replacing: .appSettings)` — atalho que todo app de macOS tem,
  morto, é pior que a janela que ele abria.
- 2026-08-28: a janela tem tamanho FIXO (520×620), não `.contentSize` livre: as duas
  abas têm alturas naturais diferentes e a janela pularia de tamanho a cada troca.
- 2026-08-28: `home` entrou no `PREFIXES` do `check-strings.sh`. A lista é o registro de
  namespaces de chave; superfície nova pede prefixo novo, senão a chave vira "literal
  solto" e "tradução órfã" ao mesmo tempo.
- 2026-08-26: Ajustes e Grupos ganharam pontes mútuas; a unificação em uma janela com abas fica para decisão pré-venda. *(revertido em 28/08: as pontes eram o defeito.)*
- 2026-08-26: "—" virou "pronta" com tooltip para conta sem amostra; % com idade >1h esmaece com tooltip da idade.
- 2026-08-26: menu ⋯ por conta (Relogar/Remover) — relogin reusa a mesma casa via `LoginSheet` em modo relogin.
- 2026-08-28: a linha de conta mostra as DUAS janelas (`5h X%  7d Y%`), não só a que decide. Primeira tentativa foi rotular apenas a vencedora — resolvia a ambiguidade do número solto e criava outra, porque sumia da tela a pergunta que se faz primeiro ("quanto me resta agora?"). A janela que manda se distingue por peso e cor; a barra desenha ela. View compartilhada: `Panel/WindowReading.swift`.
- 2026-08-28: a seção CONTAS virou TABELA. A primeira versão das duas janelas era
  uma lista de linhas independentes: cada linha repetia "5h" e "7d", e como só a
  largura do NÚMERO era fixa (não a do par rótulo+número, nem a da barra, ausente
  em conta sem amostra), nada alinhava de conta para conta. Agora o rótulo da
  janela sobe uma vez para o cabeçalho, as larguras vivem em `AccountsLayout`
  (usado pelo cabeçalho, pelas linhas e pelo bloco "pronta", que ocupa a largura
  das três colunas), a conta ativa ganha fundo próprio, o tooltip é da linha
  inteira e o traço de janela ausente foi para `.quaternary` — repetido na coluna
  toda, com peso de número ele era a coisa mais visível da tela. `WindowReading`
  ganhou `Layout` (.column no painel, .labeled em Grupos, onde o cartão não tem
  coluna acima para ancorar o número). A seção também passou a usar o mesmo
  cartão de vidro de Sessão e Valor, no lugar do `Divider`.
- 2026-08-28: formatação de % centralizada em `UsagePercent` (CCUsageCore). Havia TRÊS conversões divergentes: o sensor e `GroupsView` arredondavam, `Format.percent` truncava — 0,666 virava 67% numa tela e 66% na outra.
- 2026-08-28: amostra com mais de 12h ganha ícone de relógio com tooltip (esmaecer, sozinho, não comunicava o risco de número otimista em conta compartilhada).

- 2026-08-28: a MiniBar desenha a janela de 5h, não a que decide. Ela encosta na coluna "5h" e o olho casa gráfico com número vizinho — cheia ao lado de um "—" ou de um "2%", ela mentia. Sem medida válida a trilha fica vazia (uma barra mínima leria como "quase zero", que é afirmação).
- 2026-08-28: clicar numa conta da tabela troca o cartão de baixo pelo detalhamento DELA (5h e 7d com barra e reset, idade da amostra). Clicar de novo, ou o ×, volta para a sessão. O cartão de conta é de propósito mais pobre: tokens/min e valor vêm do JSONL do perfil local e não existem para as outras contas — preenchê-los com o número do perfil medido atribuiria a ela consumo alheio.

- 2026-09-18: o alvo do app estava com **60 erros e não construía** desde que o Command Line Tools subiu para 27.0 — todo `@State` virou erro de macro. O app instalado era de 28/08 e os últimos commits nunca tinham sido compilados. Saída: `ViewState.swift`. `./Scripts/test.sh` falhava junto, porque `swift test` constrói todos os alvos.
- 2026-09-18: `App.init()` chama `router.healShellIntegration()` logo após setar o `routerPath` — é o único momento em que se sabe onde o binário está AGORA.

- 2026-09-18: o **panorama saiu inteiro** (`Panorama`, `PanoramaStore`, `RouterTypes`, o caminho legado do `AccountsSection`, o fallback do `MenuBarLabel`). Era a leitura do mundo cswap/teamclaude, de antes dos grupos: com grupos criados nunca era desenhado, e mesmo assim o store lançava um `python3` a cada 60s — que chamava o `cswap`, que faz polling de `api.anthropic.com/api/oauth/usage` com o token OAuth. O invariante "o app não faz chamada de rede" era verdade do código dele e falso do processo dele. ~600 linhas a menos.

- 2026-09-18: o cartão de conta ganhou a janela POR MODELO, embaixo e com carimbo próprio — ela vem da sonda e pode ser de ontem enquanto as duas de cima são de agora. Na tabela, quando é o modelo que manda, entra o `ModelBadge`: sem ele a linha mentiria por omissão, porque nem a coluna de 5h nem a de 7d ficam destacadas e o usuário veria duas folgas sem nada explicando a troca.
- 2026-09-18: botão "Medir contas" por grupo, com spinner. Fica na UI e não no laço porque cada conta custa um Node e uma requisição — é ação do usuário, com resposta visível enquanto roda.

- 2026-09-18: `SessionsBadge` no cabeçalho de grupo, nas duas superfícies. Ponto cheio quando alguma sessão está trabalhando ou esperando você; vazio quando estão todas ociosas — a diferença entre "há sessões abertas" e "há trabalho em curso" é o que decide se trocar a conta agora se faz sentir.

- 2026-09-19: rótulo, ícone e tooltip da medida acompanham a **origem**: antena para o sensor (uma requisição que a conta atendeu), medidor para a sonda (uma consulta feita de propósito). O tooltip da linha ganhou uma frase dizendo qual das duas, porque as duas são oficiais e dizem coisas diferentes — o sensor não vê o que os colegas gastaram depois.

- 2026-09-22: **"Mostrar no Dock"**. O app é um widget de barra, e numa barra cheia (dezenove itens, tela com notch) o macOS esconde o que não cabe **sem avisar** — medido: com o monitor externo desconectado o item some inteiro. Sem Dock, sem ⌘-Tab e sem janela, o app fica rodando sem nenhuma superfície por onde ser aberto. A política de ativação é trocada em tempo de execução; o `AppDelegate` também trata o clique no ícone do Dock, porque ícone que não abre nada é pior que ícone nenhum.
- 2026-09-22: a janela abre sozinha no lançamento quando não há grupo nenhum (primeira execução) ou quando o Dock está ligado, via `defaultLaunchBehavior`. Decidido uma vez, no lançamento: reavaliar faria a janela reaparecer sozinha no meio do uso.
- 2026-09-22: `RouterConfigStore.init` passou a chamar `refreshUsage()`. A etiqueta do menu bar é desenhada antes do primeiro laço, e com `activeByGroup` vazio ela saía sem o nome da conta — que é a única coisa que ela existe para dizer. O rótulo vive num `NSStatusItem`, onde o rastreamento de observação não é confiável, então não se corrigia depois.

## Pendências conhecidas
- A etiqueta do menu bar (ícone + % + nome) custa ~70 pontos. Em barra disputada ela é a primeira a sair, e não há API para saber que isso aconteceu. O Dock é a saída; encurtar a etiqueta é a alternativa, ao custo de tirar da barra justamente o nome da conta.
- O detalhamento por conta não tem tokens/min nem valor (o sensor não colhe isso); só sairia da medição local, que é de um perfil só.
- Seção Sessão/Valor mede só o perfil padrão; medir por grupo é evolução futura.
- `setNickname` existe no store sem UI correspondente.
