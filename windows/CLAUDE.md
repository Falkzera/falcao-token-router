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
cd app; npm run dev       # front no navegador, com backend simulado
cd app; $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"; npx tauri dev   # o app
```

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
