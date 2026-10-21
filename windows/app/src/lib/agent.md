# app/src/lib — a ponte com o backend e o que as telas dividem

## Arquivos
- `api.ts` — as chamadas ao backend (`app_info`, `get_snapshot`, `fit_flyout`, `open_home`,
  `quit_app`, as ações de grupos e contas, a integração de terminal — `terminal_report`,
  `install_integration`, `allow_profiles_for` — e os ajustes — `get_settings`, `set_autostart`,
  `set_show_in_taskbar`, `set_hidden_summary`, `open_url` —, o histórico — `get_history`,
  `set_scheduled_measure` —, a status line — `get_status_line`, `set_status_line`,
  `test_status_line` —, o login — `start_login`, `start_relogin`,
  `current_login`, `login_submit_code`, `login_retry`, `login_recheck`, `login_close`) e os
  eventos (`navigate`, `snapshot-changed`, `login-changed`). Dentro do
  Tauri, `invoke`/`listen`; no navegador, o backend simulado. `currentView()` pelo rótulo da
  janela (ou `?view=`); `initialSelection()` abre o cartão de uma conta só no navegador
  (`?select=`).
- `mock.ts` — o backend simulado: os mesmos comandos, com cenário, idioma, aba e seleção pela
  URL (`?view=flyout&state=uso|vazio|pronta|critico|erro&lang=pt-BR&select=A2`), a integração
  (`&terminal=ausente|ok|bloqueado|parcial|velha|semrouter&devmode=1&install=falha&diretiva=1`,
  com as mesmas contas do `view` do Rust e o 5.1 em `Restricted` de fábrica depois do Ativar) e
  os ajustes (`&autostart=falha&taskbar=1&resumo=fiveHourReset,model` — o que o resumo das
  contas não mostra; `open_url` recusa o que o Rust recusa), a status
  line (`&statusline=itens|vazia|comando&runner=powershell|nenhum&teste=ok|colorido|vazio|
  falha|prazo|naosubiu|semshell`; a prévia é uma imitação do `render` do núcleo só para o
  navegador, e a escolha gravada volta normalizada como no Rust) e o login
  (`&login=ok|codigo|duplicada|errada|recusado|encerrado|timeout|semclaude`, com as fases no
  tempo e a revisão crescente). Dados só de exemplo (`@exemplo.com`, Acme, `C:\Users\exemplo`).
  A aba pela URL inclui `&tab=history`; o histórico vem do `mock-history.ts`.
- `mock-history.ts` — o `get_history` do backend simulado (21/10/2026): 4 semanas de pontos de 5
  em 5 horas para cada conta do quadro, estáveis por conta e por bloco de 5 h (nada sorteado:
  a releitura não muda o gráfico); a 5h alta no horário de trabalho e sem janela de madrugada,
  a semanal subindo até o reset de segunda 9:00. Séries montadas como no Rust (recorte, buracos
  de mais de 7h30, marcas). Cenários `&historico=cheio|vazio|lacuna|desligado`.
- `LineChart.svelte` — o gráfico de linha do Histórico (21/10/2026), SVG à mão: eixo vertical
  FIXO de 0 a 100%, marcas de tempo vindas do backend, o limiar tracejado com rótulo, linha de
  2 px na cor `--chart-line` (validada contra as duas superfícies), bolinha só no último ponto,
  no ponto isolado e no escolhido; a dica segue o ponteiro até o ponto mais perto no tempo (lado
  com mais espaço) e o teclado (foco: o último; ←/→, Home/End, Esc); `aria-live` e uma tabela
  escondida com todos os pontos para leitor de tela. Grupo focável com `svelte-ignore` (vírgula
  entre os códigos — o Svelte 5 ignora o resto depois de um espaço).
- `types.ts` — espelho dos `#[derive(Serialize)]` do Rust (camelCase), inclusive o `Snapshot` e o
  `HistoryView`.
- `i18n.ts` — `t(chave, …args)`, `format` (`%@`, `%d`, `%1$@`, `%%`, como no macOS),
  `setLocale` (uma vez, na subida).
- `format.ts` — duração ("1h 12m"), quanto falta com dias (`untilText`: "4d 13h" a partir de um
  dia), "reseta seg (28) 9:00 · em 4d 13h" (`resetText`: o "quando" vem escrito do núcleo, o que
  falta é contado de agora), `notStarted` (5h em 0% sem reset = a janela não começou — o
  `/usage` do Claude Code 2.1.281 só escreve o "· resets" com a data, e a de 5h sem uso não tem;
  as semanais têm sempre), semáforo (0,66/0,90) e os limiares de idade (1 h esmaece, 12 h
  relógio).
- `clock.svelte.ts` — o relógio da tela, andando a cada 30 s (idades não congelam na tela aberta).
- `Icon.svelte` — os ícones do app em SVG (grupos, ajustes, fechar, relógio, sensor, sonda,
  terminal, copiar, ✓, lápis, +, aviso, informação).
- `Modal.svelte` — o `<dialog>` nativo (`showModal`): foco preso, Esc fecha, clicar fora NÃO fecha
  (confirmação destrutiva não pode sumir por clique errado).
- `Menu.svelte` — o menu ⋯ (fecha ao escolher, ao clicar fora e no Esc; itens destrutivos em
  vermelho, desabilitados em cinza).
- `Switch.svelte` — o interruptor do Windows 11, CONTROLADO: mostra o que o backend confirmar,
  não o clique (um "Abrir no login" recusado volta a desligado). `Rich.svelte` — crase do
  catálogo vira `<code>`.
- O backend simulado registra as chamadas em `window.__mockCalls` (para a conferência contar,
  por exemplo, que o limiar grava UMA vez).
