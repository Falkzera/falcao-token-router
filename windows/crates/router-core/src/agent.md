# router-core/src — o motor do porte (≙ Sources/CCUsageCore, parte do router)

Sem UI e sem rede. Lê e escreve os MESMOS arquivos do app macOS (`config.json`,
`usage/<email>.json`) e reproduz as regras do motor uma a uma.

## Arquivos
- `lib.rs` — módulos e reexports (superfície plana como no Swift).
- `ids.rs` — `Id`: UUID que serializa em MAIÚSCULAS (o `uuidString` do Swift), lê qualquer caixa.
- `time_fmt.rs` — datas ISO-8601 **sem fração** (o decodificador `.iso8601` do Swift recusa fração).
- `engine/` — modelos, credencial, rotação, store, lançador, sessões, integração de terminal.
- `platform/` — o específico do Windows (E/S com nova tentativa, mutex, FILETIME, links, 8.3…).
- `usage/` — `usage_percent`, resolvedor do `claude` e a sonda `/usage`.

## Padrões
- Comentário explica o PORQUÊ, com a data do que foi observado no Claude Code.
- Erros são fatos tipados (`thiserror`); o texto de UI fica com o app (catálogo en/pt-BR).
- Tudo que toca o disco do usuário é injetável (home, leitor de sessões, executor da sonda)
  — nenhum teste toca `%USERPROFILE%\.claude`.
