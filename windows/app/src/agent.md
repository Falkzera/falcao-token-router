# app/src — o front (Svelte 5 + TypeScript)

Uma página para as duas janelas: `App.svelte` decide a superfície pelo rótulo da janela (`home`
ou `flyout`; no navegador, `?view=`). Runes do Svelte 5 (`$state`, `$derived`, `$props`).

## Arquivos
- `main.ts` — monta o `App` e carrega o CSS base.
- `App.svelte` — espera o `app_info` (idioma do Windows) antes de desenhar qualquer coisa; marca
  `html[data-view]` (o flyout tem altura do conteúdo, sem rolagem) e `html[data-host]`
  (`browser`: a janela Grupos/Ajustes simulada numa moldura 520×620; no app ela é do usuário e
  o corpo a acompanha).
- `panel/` — o flyout da bandeja. `home/` — a janela Grupos/Ajustes. `lib/` — ponte com o
  backend, tipos, i18n, formatação, relógio, ícones, backend simulado. `locales/` — os
  catálogos en/pt-BR. `styles/app.css` — tokens claro/escuro (inclusive as cores de uso) e a coluna das páginas da
  janela (`--page-inline`).

## Padrões
- **Toda string de UI vem do catálogo**, pela função `t("chave", …args)`; as chaves são as do
  macOS quando o conceito é o mesmo, com o mesmo formato de placeholder. O
  `scripts/check-strings.mjs` barra o resto (inclusive texto solto na marcação).
- Chave de catálogo sempre LITERAL no código (nada de `` `groups.error.${x}` ``): é o que deixa o
  verificador achar as chaves usadas.
- O front nunca chama `invoke` direto: tudo por `lib/api.ts`.
- Percentual nunca é calculado aqui: vem pronto do backend.
