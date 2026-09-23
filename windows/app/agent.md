# app — o app Tauri (≙ Sources/FalcaoTokenRouter)

Tauri v2 + Svelte 5 + TypeScript + Vite. Apresentação e orquestração: a regra de negócio vem do
`router-core`, a marca do `gauge-mark`. Fase 5 do porte, em fatias.

## Estrutura
- `package.json` / `package-lock.json` — versões FIXAS (aprovadas em 22/09/2026: svelte 5.57.1,
  vite 8.3.0, vite-plugin-svelte 7.3.0, svelte-check 4.7.6, typescript 6.0.3 — o 7 não é aceito
  pelo svelte-check —, @tsconfig/svelte 5.0.8, @tauri-apps/cli 2.11.5, @tauri-apps/api 2.11.1).
  Dependência nova só com o usuário de acordo.
- `vite.config.ts` (porta fixa 1420, alvo Chromium), `svelte.config.js`, `tsconfig.json`
  (estrito, `noUncheckedIndexedAccess`), `index.html` (uma página; a janela decide a superfície).
- `src/` — o front (ver o `agent.md` de lá). `scripts/check-strings.mjs` — as strings.
- `src-tauri/` — o crate Rust do app (membro do workspace de `windows/`).

## Comandos (a partir de `windows\app`)
```powershell
npm run dev            # só o front, no navegador, com o backend simulado (lib/mock.ts)
npm run check          # svelte-check (avisos = erro) + check-strings
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"; npx tauri dev   # o app de verdade
```
O `tauri dev` precisa do `cargo` no PATH (nesta máquina ele não está por padrão).

## Decisões
- 22/09/2026: sem plugin JS nenhum — o front só fala com comandos do próprio app (`lib/api.ts`),
  o que deixa o backend simulado do navegador do mesmo tamanho que a API real.
- 22/09/2026: o `cargo` compila o app SEM o `dist` do front (em modo dev o Tauri usa o
  `devUrl`); a CI roda Rust e front como passos independentes. O pacote de verdade sai do
  `scripts\build.ps1` (23/09/2026): `tauri build` com o config do instalador, que roda o
  `npm run build` (o `dist`) antes do Rust.
