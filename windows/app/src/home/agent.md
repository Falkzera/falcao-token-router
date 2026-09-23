# app/src/home — a janela Grupos / Ajustes (≙ Settings/HomeWindow.swift + GroupsView.swift)

## Arquivos
- `Home.svelte` — as duas abas, **Grupos** (o produto) e **Ajustes** (sem medidor nesta entrega).
  O tamanho é da janela (abre no último, 520×620 na 1ª vez; redimensiona e maximiza — o Rust
  cuida, `home_window.rs`), nunca do conteúdo: as abas têm alturas naturais diferentes e a
  janela pularia a cada troca. Numa janela larga, abas, páginas e o aviso de erro seguem a mesma
  coluna (`--page-inline`: até 880 px, centrada; 16 px de margem no tamanho de sempre). A bandeja pode pedir a aba (`navigate`); a aba da abertura
  vale ANTES do 1º desenho (montar Grupos por um instante pediria o quadro do terminal à toa).
  Dona do diálogo do login ("Adicionar conta" e "Relogar…"): fica com a visão de revisão maior
  (evento × resposta do comando) e reabre o login em andamento quando a janela volta.
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
- `SettingsView.svelte` — a aba Ajustes: a status line (em cima), o resumo dos grupos, "Abrir no
  login" (do sistema, com o motivo da recusa), "Mostrar na barra de tarefas" com a explicação, a
  dica do ícone escondido no `^` (→ `ms-settings:taskbar`) e a versão.
- `SummarySection.svelte` — "Resumo dos grupos" (pedido de 23/09/2026): o que cada conta mostra
  na aba Grupos, em pares por janela (uso | reset: 5 h, 7 dias, limite do modelo); de fábrica,
  tudo. O reset fica desligado sem a janela dele (o estado é guardado e volta com ela); "Mostrar
  tudo" quando algo foi tirado. Estado local na hora do clique e gravação em FILA
  (`set_hidden_summary`). O tooltip da linha não muda: é o detalhe completo.
- `StatusLineSection.svelte` — a status line das sessões dos grupos: a chave "Usar a status line
  do app"; ligada, a PRÉVIA (vinda do backend, desenhada pelo código da CLI; num fundo de
  terminal escuro nos dois temas — as cores da linha são para ele) e os 10 itens, com
  "Restaurar a completa"; desligada, "Seu comando" (grava ao sair do campo ou no Enter), o
  shell que o roda e o prazo, e "Testar" (desligado sem shell ou sem comando), com a linha que
  ele imprimiu ou o porquê da falha (e o começo do stderr). O resultado vale para o comando
  testado: editar o campo ou trocar de modo o apaga, e um que chega depois de uma edição não
  aparece. Grava na hora, em FILA (sair do
  campo clicando na chave grava o comando e depois o modo — a última vence).
- `LoginDialog.svelte` — o login oficial (≙ LoginSheet): iniciando; o link (o navegador já
  abriu nele) com copiar, "Abrir no navegador" e "Pediu um código?" (aviso de código
  incompleto); conferindo; adicionada/renovada; duplicada e conta errada (com "Sair no
  claude.ai" e "Tentar de novo"); falha com o motivo (o do `claude`, verbatim, quando ele
  recusa); tempo esgotado com "Conferir de novo" (o spinner eterno do macOS). Fechar e Esc
  cancelam; a limpeza do disco é do backend.
- `AccountItem.svelte` — uma conta: alça (arrasta; com foco, ↑/↓ movem), ponto da ativa, rótulo e
  organização, selo do modelo (com o reset dele embaixo), relógio de amostra velha, 5h/7d
  rotulados (só a janela que manda com peso e cor), cada janela com o reset embaixo (`↻ 22:30`,
  `↻ seg (28) 9:00`; a de 5h sem uso, "não iniciada"; terciário — na altura que nome e
  organização já ocupam, a linha não cresce), "Usar" (na ativa o botão só some — o espaço fica e
  os números alinham) e o menu ⋯ (mover, relogar, remover). O que o resumo mostra é escolha do
  usuário (`hidden`, de Ajustes → Resumo dos grupos; de fábrica, tudo); o tooltip segue com
  tudo. (23/09/2026: os resets na linha nasceram como teste e ficaram, com a escolha.)
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
- Status line (23/09/2026): no navegador (backend simulado), nos dois idiomas e temas — itens
  (uma gravação por clique, a prévia muda, "Restaurar a completa"), linha vazia, modo comando
  (grava ao sair do campo; Enter com o mesmo texto não regrava), os desfechos do "Testar" e o
  sem-shell (Testar desligado). No app do SANDBOX: a prévia vem do Rust, o arquivo cai na base
  do sandbox, o "Testar" roda pelo Git Bash de verdade, e o `router.exe` de dev lê a mesma
  escolha (modo comando e linha do app). As duas páginas do app (janela e flyout) têm o mesmo
  título e endereço: dirigir a janela pela que tem `[role=tablist]`.
- Chrome headless pelo DevTools Protocol (script no scratchpad da sessão): arrasto real,
  teclado, limiar (1 gravação ao soltar), renomear, medir com spinner, diálogos e menus, nos dois
  temas e idiomas. Integração (23/09): nunca instalada → Ativar → "Instalando…" → "Instalada ✓"
  com o 5.1 em `Restricted` → Permitir com confirmação → pronta; parcial, app movido, sem
  router, diretiva de grupo, instalação que grava só parte; Ajustes com o registro recusado.
- NUNCA clicar "Ativar" nem "Abrir no login" no app de verdade: os `$PROFILE` e o `HKCU\…\Run`
  são os reais mesmo no sandbox.
- Login (23/09): todos os estados no navegador (`&login=…`), nos dois idiomas e temas; e o
  caminho real no app do sandbox com o `fake-claude` (ver `src-tauri/src/agent.md`).

## Pendências
- O login com o `claude` e o navegador de verdade: teste ponta a ponta (fase 7).
