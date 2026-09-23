# app/src/home — a janela Grupos / Ajustes (≙ Settings/HomeWindow.swift + GroupsView.swift)

## Arquivos
- `Home.svelte` — as duas abas, **Grupos** (o produto) e **Ajustes** (sem medidor nesta entrega).
  Tamanho fixo 520×620 (definido no Rust): as abas têm alturas naturais diferentes e a janela
  pularia de tamanho a cada troca. A bandeja pode pedir a aba (`navigate`); a aba da abertura
  vale ANTES do 1º desenho (montar Grupos por um instante pediria o quadro do terminal à toa).
- `GroupsView.svelte` — a aba do produto: cabeçalho com "Novo grupo", vazio com convite, os
  cartões, a seção da integração de terminal (depois dos grupos) e o aviso da última falha
  (`ErrorBanner`) embaixo. É dona do quadro do terminal (lento: pedido ao abrir, depois de cada
  ação e quando a janela volta ao foco, no máximo a cada 30 s) e diz aos cartões se a integração
  está ok / falta instalar / pede atenção.
- `GroupCard.svelte` — um grupo: renomear no lugar, sessões, selo "padrão", menu ⋯ (tornar/deixar
  de ser padrão, apagar), o comando do terminal com copiar (✓ por 2 s) e o aviso da integração
  (um link que rola até a seção), auto-troca, limiar (o número acompanha o arrasto; GRAVA SÓ AO
  SOLTAR), a lista reordenável e o rodapé (adicionar conta, "Medir contas" com spinner — uma
  medição por vez).
- `TerminalSection.svelte` — a integração de terminal (≙ TerminalIntegrationRow): "Conferindo…"
  até o quadro chegar; convite + "Ativar"; depois, uma linha por shell (PS 7, PS 5.1, Git Bash)
  com ✓/⚠ e a correção de cada problema — "Ativar" só quando instalar resolve (`needsInstall`),
  "Permitir (RemoteSigned para o seu usuário)" com confirmação para a política (escondido se uma
  diretiva de grupo vencer), a linha do `.bashrc` com copiar para o `.bash_profile` que o ignora,
  a função `claude` do usuário encadeada (info), o app movido (scripts velhos) e a dica do Modo
  de Desenvolvedor (→ `ms-settings:developers`). "Instalada ✓" por 2 s só com `ok`.
- `SettingsView.svelte` — a aba Ajustes: "Abrir no login" (do sistema, com o motivo da recusa),
  "Mostrar na barra de tarefas" com a explicação, a dica do ícone escondido no `^` (→
  `ms-settings:taskbar`) e a versão.
- `AccountItem.svelte` — uma conta: alça (arrasta; com foco, ↑/↓ movem), ponto da ativa, rótulo e
  organização, selo do modelo, relógio de amostra velha, 5h/7d rotulados (só a janela que manda
  com peso e cor), "Usar" (na ativa o botão só some — o espaço fica e os números alinham) e o
  menu ⋯ (mover, relogar, remover).
- `NewGroupDialog.svelte` — nome e a escolha "usar como grupo padrão", DESMARCADA quando o
  `~\.claude` tem um login que o router não conhece — e, se marcada, o aviso de qual.
- `ConfirmDialog.svelte` — confirmação destrutiva genérica; o texto diz o que de fato acontece.
- `ErrorBanner.svelte` — o erro da última ação (fato com código → chave do catálogo, uma por
  código) e "Dispensar" (caminho longo quebra em qualquer ponto — não empurra o botão para fora).

## Decisões (22/09/2026) — os defeitos do macOS que não vieram
- Reordenar que funciona (arrasto com captura do ponteiro + teclado + "Mover para cima/baixo").
- Limiar grava ao soltar (`change`), não a cada passo (`input`).
- "Apagar grupo" conta só as contas EXCLUSIVAS (as que de fato perdem o login).
- "Tornar padrão" confirma, e avisa do login estranho no `~\.claude` quando há um.
- Texto com crase do catálogo vira `<code>` nos diálogos (`lib/Rich.svelte`, sem HTML) e sai dos
  tooltips — o SwiftUI fazia isso sozinho (markdown do LocalizedStringKey).
- 23/09/2026: "Instalada ✓" só com a instalação inteira gravada (o macOS mostrava o ✓ com
  falha); os textos da integração falam de Windows (terminal novo, sem `source ~/.zshrc`).

## Conferência
- Chrome headless pelo DevTools Protocol (script no scratchpad da sessão): arrasto real,
  teclado, limiar (1 gravação ao soltar), renomear, medir com spinner, diálogos e menus, nos dois
  temas e idiomas. Integração (23/09): nunca instalada → Ativar → "Instalando…" → "Instalada ✓"
  com o 5.1 em `Restricted` → Permitir com confirmação → pronta; parcial, app movido, sem
  router, diretiva de grupo, instalação que grava só parte; Ajustes com o registro recusado.
- NUNCA clicar "Ativar" nem "Abrir no login" no app de verdade: os `$PROFILE` e o `HKCU\…\Run`
  são os reais mesmo no sandbox.

## Pendências
- Adicionar conta e Relogar: fatia 5.5 (login por ConPTY).
