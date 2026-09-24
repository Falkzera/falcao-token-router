# Falcão Token Router — porte Windows (Rust + Tauri)

Porte do **router** para Windows. Lê e escreve os mesmos arquivos do app macOS
(`config.json`, `usage/<email>.json`), com as mesmas regras de motor. O medidor
herdado (custo por JSONL, preços, alertas) **não** entra neste porte.

## Comandos (rode a partir de `windows/`)
```powershell
# o cargo pode não estar no PATH desta sessão; use o caminho completo se preciso:
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build --workspace
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build -p fake-claude  # os testes o rodam; `cargo test` não gera o .exe
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --workspace
.\scripts\test.ps1        # fmt + clippy -D warnings + test + svelte-check/check-strings (= CI)
.\scripts\build.ps1       # o instalador NSIS (router.exe como sidecar), conferido (= CI)
cd app; npm run dev       # front no navegador, com backend simulado
cd app; $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"; npx tauri dev   # o app
```
O instalador sai em `target\release\bundle\nsis\`; o 1º build baixa o NSIS e o bootstrapper
do WebView2 para `%LOCALAPPDATA%\tauri`. `README.md` (inglês) é o guia de quem instala.

## Mapa
- `crates/router-core` = ≙ `Sources/CCUsageCore` (parte do router). Sem UI, sem rede.
- `crates/router-cli`  = ≙ `Sources/router` → `router.exe`.
- `crates/fake-claude` = `claude` de mentira dos testes de integração.
- `crates/gauge-mark`  = o anel (bandeja + ícone do app), testado por pixel.
- `app/`               = ≙ `Sources/FalcaoTokenRouter`: Tauri v2 (`src-tauri/`) + Svelte 5 (`src/`).
- Fonte macOS portada: `Sources/CCUsageCore/{Engine,Usage}`, `Sources/router/main.swift`,
  `Tests/CCUsageCoreTests/*`. Fatos do Windows: `docs/PLATFORM.md`.

## Regras de código
- Comentários e `agent.md` em **pt-BR**; identificadores em inglês; strings de UI (fase 5)
  só via catálogo en/pt-BR (en = base).
- **Nada de chamada de rede.** O uso vem do cliente oficial (sensor + sonda).
- Credencial é **blob opaco** (`Vec<u8>` validado estruturalmente, nunca decodificado);
  nenhum tipo tem campo de refresh token.
- Datas das amostras em ISO-8601 **sem fração** (o decodificador do Swift recusa fração).
- UUID serializa em MAIÚSCULAS (como o `uuidString` do Swift).
- Descobertas do Claude Code têm comentário com o PORQUÊ e a data — custa caro redescobrir.
- Testes NUNCA tocam `%USERPROFILE%\.claude`: home, base (`ROUTER_APP_SUPPORT`) e o `claude`
  (`ROUTER_CLAUDE_BIN`) vão para um sandbox, e `CLAUDE_CONFIG_DIR` é removido do filho.
- Texto com `\` (caminhos do Windows) se edita pelo editor, não por heredoc de shell — o
  heredoc já comeu barras e virou caractere de controle em `agent.md`.
- O `router.exe` entra no instalador pelo `app/src-tauri/tauri.installer.conf.json`, NUNCA
  pelo `tauri.conf.json`: o `build.rs` do Tauri copia o `externalBin` em todo `cargo build`
  do app (quebra a CI e troca o `router.exe` de debug que os testes da CLI rodam).
- Instalar o app nesta máquina só com o usuário de acordo; o instalado sobe com o ambiente
  do sandbox (`ROUTER_APP_SUPPORT`, `USERPROFILE` falsos) até o teste ponta a ponta.

## Achados do Windows que moldam o porte (spike, 22/09/2026)
- Troca a quente vale: escrever `<perfil>\.credentials.json` com **mtime novo**
  (temp+rename) faz a sessão viva servir a nova conta no próximo request.
- Status line roda por **Git Bash**; o **stdin pode nunca fechar** (leitor com prazo).
- 1º render vem **sem** `rate_limits` → não gravar amostra vazia.
- `/usage`: data com **vírgula** (`MMM d, h:mma`), `·`=U+00B7, CRLF; deslogado = exit 0
  sem linhas `Current`.
- Função do PowerShell engole o `--` do `$args`; o perfil do usuário pode já ter uma
  `function claude` (a integração a encadeia, não a substitui).
- Login (23/09/2026): `claude auth login` num ConPTY. O `portable-pty` cria o ConPTY com
  `INHERIT_CURSOR` — ele abre pedindo a posição do cursor (`ESC[6n`) e espera a resposta; o
  hyperlink OSC 8 do link é re-emitido pelo ConPTY; e o ambiente base do `portable-pty` vem
  também do REGISTRO (`env_clear` antes do ambiente filtrado). Sucesso = `Login successful.`
  e o processo sai sozinho; o desfecho vale só conferido no disco.
- Instalador (23/09/2026): cada sessão de `claude <grupo>` mantém um `router.exe` rodando, e
  exe em execução não se sobrescreve nem se apaga — mas se RENOMEIA. Os ganchos do NSIS
  (`app/src-tauri/installer-hooks.nsh`) o tiram do caminho; sem eles a atualização silenciosa
  pulava o arquivo e dizia sucesso.
- Status line (23/09/2026, JS do 2.1.280): roda pelo executor dos hooks — `bash -c` com a pasta
  do bash na frente do `PATH` (1º termo `.sh` ganha `bash `); sem Git Bash, `pwsh`/`powershell`
  com `-ExecutionPolicy Bypass`; JSON + `\n` e `end()` no stdin (o spike nunca viu o EOF chegar
  — o prazo do sensor fica); só mostra saída de código 0. O modo "meu comando" do router
  (`statusline.json`, escolha só do Windows) reproduz isso fechando de fato o stdin do comando,
  com prazo de 5 s e a árvore do comando num Job Object.
- Sandbox com `USERPROFILE` falso: a home precisa de `AppData\Local` e `AppData\Roaming`, senão
  o `SHGetKnownFolderPath` falha e o WebView2 grava em `<exe>.WebView2` ao lado do exe.
