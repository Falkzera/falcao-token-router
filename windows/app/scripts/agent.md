# app/scripts

- `check-strings.mjs` — ≙ `Scripts/check-strings.sh`: chave usada (TS, Svelte e o Rust da
  bandeja) presente nos dois catálogos; nenhuma tradução órfã; chaves só nos prefixos
  `panel|settings|format|groups|home|tray` (sem `alerts`: o medidor não entra no porte; `tray` é
  superfície nova); placeholders iguais entre en e pt-BR (novo: uma tradução que perde um `%@`
  engole um argumento); nenhum texto solto na marcação Svelte (entre tags ou em
  `title`/`placeholder`/`aria-label`/`alt`/`label`). Roda no `npm run check` e no `test.ps1`.
