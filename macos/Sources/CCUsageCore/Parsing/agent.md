# Parsing — agent.md

## Propósito
Ler o que o Claude Code deixa no disco (`~/.claude/projects/**/*.jsonl`) e transformar em eventos, **de forma incremental** — os arquivos crescem sem parar e reler tudo a cada tique não é opção.

## Arquivos
- `JSONLParser.swift` — uma linha JSONL vira `UsageEvent`, ou nada.
- `ProjectScanner.swift` — varre a árvore de projetos e lê só o delta de cada arquivo.
- `ParseCache.swift` — o estado entre execuções: até onde cada arquivo foi lido **e os eventos que essa leitura produziu**.

## Padrões
- Linha que não parseia é ignorada, nunca derruba a varredura: o formato é de outro programa, no calendário de release dele.

## Decisões recentes
- Guardar só os offsets seria **pior que não guardar nada**: no segundo lançamento o `ingest` devolveria zero eventos (tudo "já lido") e o app acordaria sem histórico, com os tetos no piso e todos os gauges saturados. Por isso o cache carrega os eventos junto.

## Pendências conhecidas
- O cache é podado pela janela de interesse (90 dias) a cada varredura; sem isso o arquivo cresceria sem limite.
