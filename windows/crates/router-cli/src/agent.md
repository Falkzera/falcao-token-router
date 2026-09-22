# router-cli — a CLI `router` (≙ Sources/router)

Binário `router.exe`. Sem argumento, o comando é `statusline` (igual ao macOS).

## Arquivos
- `main.rs` — despacho por argv. Nesta fatia só `statusline`; os outros comandos imprimem
  o uso e saem com 2.
- `statusline.rs` — o **sensor**: lê `rate_limits` do stdin (thread + prazo de 250 ms, pega
  o 1º JSON sem esperar EOF), lê a identidade do `.claude.json` do perfil, grava a amostra
  **só se houver e-mail e ao menos uma janela**, imprime a linha colorida e sai 0 sempre.

## Decisões (22/09/2026, medidas no spike)
- stdin **nunca fecha** no Windows → leitor com prazo; nunca `readToEnd`.
- 1º render sem `rate_limits` → **não** grava amostra (corrige o defeito do macOS em que a
  amostra vazia apagava a última leitura e a conta cheia virava "pronta").
- `current_config_dir`: `CLAUDE_CONFIG_DIR` não-vazio → dedicado; senão padrão (`%USERPROFILE%\.claude`).

## Pendências (Fase 4)
- `launch` (spawn+wait, sem `exec`; handler de Ctrl+C próprio), `is-group`, `rotate`,
  `measure`, `doctor` (checagens do Windows). Testes de integração com um `fake-claude`.
