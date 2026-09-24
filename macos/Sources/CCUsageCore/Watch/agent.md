# Watch — agent.md

## Propósito
Saber que o disco mudou, sem transformar isso em trabalho demais.

## Arquivos
- `FSWatcher.swift` — observa um diretório e chama `onChange` com debounce.

## Padrões
- O debounce não é otimização: o Claude Code escreve continuamente durante uma sessão, e sem ele o app reparsearia a cada token emitido.

## Pendências conhecidas
- Nenhuma.
