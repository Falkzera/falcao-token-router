# CCUsageCore (raiz) — agent.md

## Propósito
O topo do motor: o que a UI observa e o que transforma eventos crus em tela. Nada aqui importa SwiftUI — é a regra do módulo inteiro, e é o que deixa tudo ser exercitado sem abrir janela.

## Arquivos
- `UsageStore.swift` — a fachada observável do MEDIDOR: mantém os eventos em memória, reage a mudança no disco (via `Watch/FSWatcher`), busca a fonte "ao vivo" (hoje o sensor local, não a rede) e publica um `UsageSnapshot`. É o único tipo do medidor que a UI conhece.
- `SnapshotBuilder.swift` — função **pura**: mesmos eventos + mesmo `now` = mesmo snapshot. O número oficial vence o derivado sempre que existe, porque traz a fase real da janela, que a derivação não recupera.
- `CCUsageCore.swift` — ponto de entrada do módulo.

## Padrões
- `now: Date` entra por parâmetro onde o tempo decide. Nenhuma função aqui lê o relógio por conta própria.
- Falha de leitura mantém o último snapshot bom em vez de publicar um vazio: disco indisponível não é motivo para a tela zerar.

## Decisões recentes
- 2026-09: a fonte "ao vivo" do medidor deixou de ser uma chamada de API e passou a ser a amostra do sensor (`Engine/GroupUsage`), considerada ao vivo só se for recente. O app não faz requisição nenhuma.

## Pendências conhecidas
- O nome `liveUsageEnabled` sobrou da época em que "ao vivo" significava rede; hoje significa "prefira o sensor ao cache". Renomear é dívida de clareza, não de comportamento.
- A seção Sessão/Valor mede só o perfil padrão; medir por grupo é evolução futura.
