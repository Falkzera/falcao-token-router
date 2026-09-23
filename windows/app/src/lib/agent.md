# app/src/lib — a ponte com o backend

## Arquivos
- `api.ts` — as chamadas ao backend. Dentro do Tauri, `invoke`; no navegador (`npm run dev`),
  o backend simulado. Único lugar que conhece nome de comando.
- `mock.ts` — o backend simulado: os mesmos comandos, com cenário e idioma pela URL
  (`?view=home&lang=pt-BR`), para cada estado da tela ser conferido no Chrome.
- `types.ts` — espelho dos `#[derive(Serialize)]` do Rust (camelCase).
- `i18n.ts` — `t(chave, …args)`, `format` (`%@`, `%d`, `%1$@`, `%%`, como no macOS),
  `setLocale` (uma vez, na subida, pelo idioma que o backend manda).
