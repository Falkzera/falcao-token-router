# Settings (app) — agent.md

## Propósito
A janela única, em duas abas: **Grupos** (o produto — rodízio, contas, integração de terminal) e **Medidor** (os ajustes do medidor herdado).

## Arquivos
- `HomeWindow.swift` — a janela e o `TabView`. Define `HomeTab`, o store `HomeNavigation` (qual aba abrir — a escolha vem do painel, de OUTRA cena, e como estado de view se perderia), `SettingsCommand` (o item de menu do ⌘,) e `HomeWindowID`.
- `GroupsView.swift` — a tela do produto: a lista de grupos, o login pendente e o que a
  compõe. Era um arquivo de 699 linhas com seis views dentro; quebrado em 24/09/2026 pela
  régua de 600 linhas de produção, uma view por arquivo como no `Panel/`.
- `GroupCard.swift` — o cartão de um grupo: nome editável, contas, ordem por arrastar,
  limiar, troca automática, "Medir contas" e as confirmações destrutivas.
- `AccountRow.swift` — uma conta na lista: uso por janela com a idade e a procedência da
  amostra, e o menu ⋯ (Usar, Relogar, Remover).
- `LoginSheet.swift` — a folha do login oficial (pty): link, código, e os desfechos
  (sucesso, conta errada, duplicada, falha).
- `NewGroupSheet.swift` — a folha de criar grupo.
- `TerminalIntegrationRow.swift` — a linha que instala a integração de shell num clique.
- `SettingsView.swift` — a aba do medidor: plano, sensor, alertas, teto, sistema (Dock e abrir no login).
- `LoginItem.swift` — "abrir no login" via `SMAppService`. O estado exibido é **lido do sistema**, nunca de uma preferência nossa: um app ad-hoc pode ter o registro recusado, e um checkbox marcado sobre registro que falhou é pior que nenhum checkbox.
- `SettingsFormState.swift` — rascunho de digitação do formulário. Fica fora de `AppSettings` de propósito: é rascunho, não preferência, e nada aqui é persistido.

## Padrões
- Toda string de UI vem do catálogo (`check-strings.sh` bloqueia o resto).
- Botão que muda estado dá feedback visível em ≤2s. Ação destrutiva SEMPRE confirma — e o texto do diálogo tem de dizer o que de fato acontece (o de remover conta afirmava que o login não era apagado; passou a apagar, e o texto mudou junto).
- Nunca `@State` — usar `@ViewState`. Ver o `agent.md` do alvo.

## Pendências conhecidas
- O comentário de `SettingsFormState` justifica sua existência por `@State` ser inatingível; desde `ViewState.swift` isso deixou de valer. A razão que sobrevive é a outra que ele já dá: rascunho não é preferência.
- `setNickname` existe no store sem UI correspondente.
- `StatusLineSection.swift` — a seção dos Ajustes que escolhe o que a status line dos grupos mostra: a prévia ao vivo e um toggle por item, com "Restaurar a linha completa".
- `ANSIText.swift` — converte a linha com escapes ANSI em `AttributedString`. Existe por correção, não enfeite: a prévia é desenhada pelo MESMO `StatusLineView.render` da sessão, então ela não pode discordar do que a sessão mostra.
