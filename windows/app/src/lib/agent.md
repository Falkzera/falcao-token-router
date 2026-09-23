# app/src/lib — a ponte com o backend e o que as telas dividem

## Arquivos
- `api.ts` — as chamadas ao backend (`app_info`, `get_snapshot`, `fit_flyout`, `open_home`,
  `quit_app`) e os eventos (`navigate`, `snapshot-changed`). Dentro do Tauri, `invoke`/`listen`;
  no navegador, o backend simulado. `currentView()` pelo rótulo da janela (ou `?view=`);
  `initialSelection()` abre o cartão de uma conta só no navegador (`?select=`).
- `mock.ts` — o backend simulado: os mesmos comandos, com cenário, idioma, aba e seleção pela
  URL (`?view=flyout&state=uso|vazio|pronta|critico|erro&lang=pt-BR&select=A2`). Dados só de
  exemplo (`@exemplo.com`, Acme).
- `types.ts` — espelho dos `#[derive(Serialize)]` do Rust (camelCase), inclusive o `Snapshot`.
- `i18n.ts` — `t(chave, …args)`, `format` (`%@`, `%d`, `%1$@`, `%%`, como no macOS),
  `setLocale` (uma vez, na subida).
- `format.ts` — duração ("1h 12m"), hora do reset, "reseta 13:20 · em 4h 6m", semáforo
  (0,66/0,90) e os limiares de idade (1 h esmaece, 12 h relógio).
- `clock.svelte.ts` — o relógio da tela, andando a cada 30 s (idades não congelam na tela aberta).
- `Icon.svelte` — os ícones do app em SVG (grupos, ajustes, fechar, relógio, sensor, sonda,
  terminal, copiar, ✓, lápis, +).
- `Modal.svelte` — o `<dialog>` nativo (`showModal`): foco preso, Esc fecha, clicar fora NÃO fecha
  (confirmação destrutiva não pode sumir por clique errado).
- `Menu.svelte` — o menu ⋯ (fecha ao escolher, ao clicar fora e no Esc; itens destrutivos em
  vermelho, desabilitados em cinza).
- `Switch.svelte` — o interruptor do Windows 11. `Rich.svelte` — crase do catálogo vira `<code>`.
- O backend simulado registra as chamadas em `window.__mockCalls` (para a conferência contar,
  por exemplo, que o limiar grava UMA vez).
