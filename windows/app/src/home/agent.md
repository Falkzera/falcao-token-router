# app/src/home — a janela Grupos / Ajustes (≙ Settings/HomeWindow.swift + GroupsView.swift)

## Arquivos
- `Home.svelte` — as duas abas, **Grupos** (o produto) e **Ajustes** (sem medidor nesta entrega).
  Tamanho fixo 520×620 (definido no Rust): as abas têm alturas naturais diferentes e a janela
  pularia de tamanho a cada troca. A bandeja pode pedir a aba (`navigate`).
- `GroupsView.svelte` — a aba do produto: cabeçalho com "Novo grupo", vazio com convite, os
  cartões, e o aviso da última falha (`ErrorBanner`) embaixo.
- `GroupCard.svelte` — um grupo: renomear no lugar, sessões, selo "padrão", menu ⋯ (tornar/deixar
  de ser padrão, apagar), o comando do terminal com copiar (✓ por 2 s) e o aviso de integração
  ausente, auto-troca, limiar (o número acompanha o arrasto; GRAVA SÓ AO SOLTAR), a lista
  reordenável e o rodapé (adicionar conta, "Medir contas" com spinner — uma medição por vez).
- `AccountItem.svelte` — uma conta: alça (arrasta; com foco, ↑/↓ movem), ponto da ativa, rótulo e
  organização, selo do modelo, relógio de amostra velha, 5h/7d rotulados (só a janela que manda
  com peso e cor), "Usar" (na ativa o botão só some — o espaço fica e os números alinham) e o
  menu ⋯ (mover, relogar, remover).
- `NewGroupDialog.svelte` — nome e a escolha "usar como grupo padrão", DESMARCADA quando o
  `~\.claude` tem um login que o router não conhece — e, se marcada, o aviso de qual.
- `ConfirmDialog.svelte` — confirmação destrutiva genérica; o texto diz o que de fato acontece.
- `ErrorBanner.svelte` — o erro da última ação (fato com código → chave do catálogo, uma por
  código) e "Dispensar".

## Decisões (22/09/2026) — os defeitos do macOS que não vieram
- Reordenar que funciona (arrasto com captura do ponteiro + teclado + "Mover para cima/baixo").
- Limiar grava ao soltar (`change`), não a cada passo (`input`).
- "Apagar grupo" conta só as contas EXCLUSIVAS (as que de fato perdem o login).
- "Tornar padrão" confirma, e avisa do login estranho no `~\.claude` quando há um.
- Texto com crase do catálogo vira `<code>` nos diálogos (`lib/Rich.svelte`, sem HTML) e sai dos
  tooltips — o SwiftUI fazia isso sozinho (markdown do LocalizedStringKey).

## Conferência
- Chrome headless pelo DevTools Protocol (script no scratchpad da sessão): arrasto real,
  teclado, limiar (1 gravação ao soltar), renomear, medir com spinner, diálogos e menus, nos dois
  temas e idiomas.

## Pendências
- Adicionar conta e Relogar: fatia 5.5 (login por ConPTY). Integração de terminal e Ajustes: 5.4b.
