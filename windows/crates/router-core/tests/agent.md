# router-core/tests — testes do motor

Portados de `macos/Tests/CCUsageCoreTests` com os mesmos cenários e fixtures anonimizadas
(`conta1@exemplo.com`, `C:\Users\exemplo`, org `Acme`), mais as regressões do Windows.
Credencial/adapter falsos em memória ou pastas temporárias — nada toca o sistema real.

## Arquivos
- `common/mod.rs` — `FakeStore` (≙ `FakeKeychain`: escrever nunca falha, registra escritas e
  remoções), `FakeAdapter` (identidade em memória por `ConfigDir.raw`; pode falhar a escrita),
  `ident`/`account`/`cred`/`ts`.
- `engine_tests.rs` — `UsagePercent`, `ConfigDir` (assimetria do `.claude.json`), `RouterConfig`.
- `sample_tests.rs` / `reader_tests.rs` — formato da amostra e o leitor (decaimento por reset,
  procedência, empate 7d > 5h).
- `adapter_tests.rs` — leitura e escrita CIRÚRGICA do `.claude.json` (ordem, chaves só-caixa,
  ilegível recusado, arquivo vazio).
- `platform_tests.rs` — `write_atomic` com mtime novo e nova tentativa com o arquivo preso.
- `credential_tests.rs` — onde a credencial mora, blob opaco e completo, mtime novo, nada
  regravado à toa, a guarda do perfil padrão.
- `rotation_tests.rs` — `RotationEngine`, um teste por regra (+ `probeConfigDir` e dois de ponta
  a ponta com arquivos reais).
- `store_tests.rs` — os 16 do `StoreTests.swift` + regressões (config ilegível, reordenar, remoção
  fora da base, sessões, integração, sonda) + o que o app usa (grupo criado dedicado, login
  estranho no `~\.claude`, contas exclusivas, medição em três passos, casa pendente descartada).
- `terminal_report_tests.rs` — o quadro da integração por shell (edições, política sem o escopo
  Process, `function claude` em UTF-8/UTF-16, `.bash_profile`, scripts atuais/obsoletos), numa
  máquina de mentira: nenhum PowerShell de verdade é consultado.
- `lock_tests.rs` — a trava entre processos (espera, dono morto, bases diferentes, nome estável).
- `launcher_tests.rs`, `provider_env_tests.rs`, `claude_binary_tests.rs`,
  `session_registry_tests.rs`, `shell_integration_tests.rs`, `profile_sharing_tests.rs`,
  `probe_tests.rs` — o resto do `LauncherTests`/`ProbeTests`/`SessionRegistryTests` do Swift e
  as regressões do Windows de cada peça.

## Padrões
- Todo bug real vira teste que nomeia o episódio ("Login expired de 26/ago", "deadlock da
  presunção de fresca", "~/.zshrc não-UTF-8 destruído").
- Fixture com dado real é anonimizada ANTES de entrar (e-mail, máquina, zona, porcentagens).
