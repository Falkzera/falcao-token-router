# scripts — verificação e empacotamento do porte

- `test.ps1` — a verificação que a CI roda: `cargo fmt --check`, `cargo clippy --workspace
  --all-targets -D warnings`, o build do `fake-claude`, `cargo test --workspace` e, no front,
  `svelte-check` + `check-strings`. Acha o `cargo` do rustup se ele não estiver no `PATH`.
  Rodar a partir de qualquer pasta.
- `build.ps1` — o instalador, igual na CI: o `router.exe` de release vira o sidecar
  (`app\src-tauri\binaries\router-<alvo>.exe`, o nome que o `externalBin` pede) e o
  `tauri build --config src-tauri/tauri.installer.conf.json` faz o resto (o front pelo
  `npm run build`, o app em release, o NSIS). No fim CONFERE: o instalador é deste build, e o
  `installer.nsi` que o NSIS compilou leva o `router.exe`, o exe com o `mainBinaryName` e os
  ganchos do `installer-hooks.nsh` — sem precisar de 7-Zip. Imprime o caminho e os tamanhos.
  Põe o `cargo` do rustup no `PATH` só durante o build (o `tauri build` também o chama).
- O `build.ps1` roda no Windows PowerShell 5.1 (conferido) e no PowerShell 7 (a CI): é UTF-8
  COM BOM, porque tem acento em mensagem e o 5.1 lê arquivo sem BOM como ANSI. O `test.ps1`
  está sem BOM — a CI o roda pelo `pwsh`, que lê UTF-8 sem BOM.

## Decisões
- 23/09/2026: o `fake-claude` é compilado ANTES dos testes — o `cargo test` não gera o .exe de
  um pacote sem testes de integração, e numa máquina limpa (a CI) os testes da CLI e do login
  o procurariam em vão (aqui ele existia de builds anteriores; conferido removendo o .exe).
- 23/09/2026: o primeiro `tauri build` baixa para `%LOCALAPPDATA%\tauri` o NSIS 3.11, o
  `nsis_tauri_utils.dll` e o `MicrosoftEdgeWebview2Setup.exe` (bootstrapper embutido); os
  seguintes usam o cache.
- 23/09/2026: o `tauri build` apaga e recria `target\release\nsis\`; um processo com o
  diretório atual lá dentro (aqui, o shell de uma ferramenta que tinha feito `cd` para ler o
  `installer.nsi`) o fez falhar com "arquivo em uso" (os error 32). Ler o `.nsi` pelo caminho,
  sem entrar na pasta.
