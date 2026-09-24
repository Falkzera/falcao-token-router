# app/src-tauri — o crate Rust do app

Membro do workspace de `windows/` (um `Cargo.lock`, um `target/`): o `test.ps1` o formata, o
passa no clippy e roda os testes dele junto com o resto.

## Arquivos
- `Cargo.toml` — `falcao-token-router` (lib `falcao_token_router_lib`); tauri 2.11 com
  `tray-icon`; plugins single-instance, autostart, opener, clipboard-manager (usados só do Rust);
  `portable-pty` 0.9 (o login oficial num ConPTY — aprovado em 22/09/2026) e o `regex` do
  workspace.
- `build.rs` — `tauri_build::build()`: embute o `icons/icon.ico` no .exe e gera `gen/` (ignorado).
- `tauri.conf.json` — `identifier` = `com.synqo.falcao-token-router` (o bundle id do macOS);
  `productName` ASCII sem espaço (`FalcaoTokenRouter`: é a pasta da instalação, que a status
  line cita) e o mesmo `mainBinaryName` (sem ele o exe instalado seria o do cargo,
  `falcao-token-router.exe`); nenhuma janela declarada (o Rust as cria); CSP fechada; NSIS
  por usuário (`%LOCALAPPDATA%\FalcaoTokenRouter`, sem admin), em inglês e português (o NSIS
  escolhe pelo idioma do Windows), com o bootstrapper do WebView2 embutido e os ganchos do
  `installer-hooks.nsh`.
- `tauri.installer.conf.json` — o `externalBin`: o `router.exe` como sidecar. Mesclado SÓ pelo
  build do instalador (`tauri build --config`, no `scripts\build.ps1`) — ver Decisões.
- `installer-hooks.nsh` — os ganchos NSIS (`installerHooks`): antes de instalar e de
  desinstalar, tiram do caminho um `router.exe` EM USO (vai para o `%TEMP%` com nome único, ou
  é renomeado na pasta) e limpam as sobras de vezes anteriores. UTF-8 com BOM, como os `.nsh`
  do Tauri (sem BOM o NSIS lê ANSI).
- `binaries/` — gerado pelo build (fora do git): `router-<alvo>.exe`, o nome que o Tauri pede.
- `capabilities/default.json` — as janelas `home` e `flyout` com `core:default` (eventos e o
  básico da janela).
- `icons/` — GERADOS pelo `icongen` do `gauge-mark` (não editar à mão).
- `src/` — o código (ver o `agent.md` de lá).

## Decisões
- 23/09/2026: o `externalBin` fica FORA do `tauri.conf.json`. O `build.rs` do Tauri
  (`tauri-build` 2.6) copia cada sidecar para `target\<perfil>\` em TODO `cargo build` do app:
  sem o arquivo (a CI, um clone novo) a compilação quebra, e com ele o `router.exe`
  recém-compilado do workspace seria trocado pela cópia do último instalador — a que os testes
  da CLI rodariam. Os testes do `system.rs` fixam o contrato (sidecar só no config do
  instalador, com o nome que o app procura).
- 23/09/2026: onde as coisas caem (conferido no `installer.nsi` gerado pelo 1º build):
  `File /a "/oname=router.exe"` no `$INSTDIR`, ao lado do exe principal — onde
  `system::router_beside` procura. O desinstalador apaga os dois, o `uninstall.exe`, os
  atalhos, a chave de desinstalação e o valor `FalcaoTokenRouter` do `HKCU\…\Run` (o nome que
  o plugin de autostart usa); a base do router (`com.synqo.falcao-router`, com as credenciais)
  nunca é tocada. Só com "apagar os dados do app" marcado ele leva também
  `%APPDATA%`/`%LOCALAPPDATA%\com.synqo.falcao-token-router` e a `HKCU\Software\synqo\…`.
- 23/09/2026: instalado de verdade nesta máquina (silencioso, sem atalhos; o app só subiu no
  sandbox): `routerFound` pelo DevTools, janela e tooltip certos. Com um processo segurando o
  `router.exe`, o modelo do Tauri PULAVA o arquivo na atualização silenciosa e saía com 0 (app
  novo, router velho) — daí o `installer-hooks.nsh`. Com ele: a atualização trocou o arquivo e a
  sessão seguiu viva; a desinstalação removeu a pasta inteira; a sobra anterior no `%TEMP%` saiu
  na vez seguinte. A foto do sistema real depois = a de antes, tirando a
  `HKCU\Software\synqo\FalcaoTokenRouter` (design do Tauri; apagada à mão) e o cache de ícones
  da bandeja do Windows.
- 23/09/2026: a 1ª webview (o flyout) caiu no `<exe>.WebView2` AO LADO DO EXE num sandbox cuja
  home falsa não tinha `AppData\Local`: o `SHGetKnownFolderPath` falha se a pasta não existe e o
  Tauri fica sem `data_directory`. Perfil real sempre a tem; o sandbox passou a criá-la.
