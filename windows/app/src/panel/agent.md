# app/src/panel — o flyout da bandeja (≙ Sources/FalcaoTokenRouter/Panel)

O painel do clique esquerdo no ícone: a tabela de contas por grupo, o detalhamento da conta
tocada e o rodapé. Sem as seções do medidor (sessão local, valor): o porte é só o router.

## Arquivos
- `Panel.svelte` — a composição; relê o quadro no evento `snapshot-changed`; mede a própria
  altura (ResizeObserver) e pede ao backend (`fit_flyout`) — a janela acompanha o conteúdo.
  Sem grupos: convite para criar o primeiro. Rodapé: as DUAS portas da mesma janela (Grupos,
  Ajustes) e Sair.
- `AccountsTable.svelte` — a TABELA (≙ AccountsSection): rótulo da janela no cabeçalho, colunas
  de largura fixa (40/34/34, respiro 8) iguais no cabeçalho e nas linhas.
- `AccountRow.svelte` — a linha inteira é alvo do clique e dona do tooltip; ponto cheio na
  ativa; só a janela que MANDA tem peso e cor; >1 h esmaece; >12 h relógio com a idade.
- `AccountDetail.svelte` + `Gauge.svelte` — o cartão da conta: 5h e 7d com barra e reset
  ("reseta 13:20 · em 4h 6m", "reseta seg (28) 9:00 · em 4d 13h" — o "quando" escrito pelo núcleo,
  o que falta contado de AGORA; a de 5h que não começou diz "começa na próxima mensagem" — o
  `session` do `Gauge`), quem mediu (antena = sensor, medidor = sonda) e
  a janela por modelo embaixo com carimbo próprio. Janela vencida = "sem medida válida".
- `MiniBar.svelte` (a barra desenha a de 5h, a coluna vizinha; sem medida a trilha fica vazia),
  `ModelBadge.svelte` (quando quem manda é a janela por modelo), `SessionsBadge.svelte` (ponto
  cheio se alguma sessão trabalha ou espera o usuário).
- `accountHelp.ts` — o tooltip da linha (≙ AccountHelp): janelas, idade, quando cada janela
  reseta (uma por linha, "5h reseta 22:30 · em 1h 12m", a do modelo também — pedido de
  23/09/2026; a de 5h sem uso diz que não há janela aberta e que ela começa na próxima
  mensagem), QUEM mediu, e o limite por modelo com a idade da sonda.

## Padrões
- **Número sem procedência não vai para a tela.** O % chega pronto do backend (`UsagePercent`
  do núcleo); a janela, a origem e a idade viajam junto.
- **Ausência não é zero**: sem amostra = "pronta"; janela vencida = "—" e trilha vazia.
- Conferido no Chrome (headless, `?view=flyout&state=uso|vazio|pronta|critico|erro&lang=…&select=<id>`)
  nos dois temas e idiomas.

## Decisões
- 22/09/2026: o × do cartão diz "Fechar" (chave nova `panel.account.close`): no macOS voltava
  para o cartão de SESSÃO, que o porte não tem.
- 22/09/2026: o tooltip nativo de uma linha sob o mouse trava a captura de tela do Chrome pela
  extensão — conferir com o mouse fora das linhas ou pelo Chrome headless.
