# doctor — agent.md

## Propósito
O `router doctor`: confere a instalação e nomeia o que está torto. Os modos de falha do
router são silenciosos — uma função de shell que não carrega não dá erro, só abre o `claude`
na conta errada —, então cada checagem diz o problema pelo nome. Imprime em português, como o
`router` do macOS.

## Arquivos
- `mod.rs` — a ordem das checagens, o relatório (`ok` / `!!`) e o código de saída.
- `terminal.rs` — a função de shell: os scripts, os dois `$PROFILE`, o `.bashrc` e a política
  de execução.
- `status_line.rs` — a status line (o sensor) rodando de verdade pelo shell que o Claude Code
  usa, a escolha do `statusline.json` e o comando do usuário, testado com uma sessão de
  exemplo.
- `accounts.rs` — quem serve cada grupo, as sessões vivas, conta ativa em dois grupos e os
  links do perfil.
- `environment.rs` — as variáveis que desviam a sessão e o `claude` instalado, com a versão.

## Padrões
- Uma checagem que falha diz o que fazer, não só o que viu.
