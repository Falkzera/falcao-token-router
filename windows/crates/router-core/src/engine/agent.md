# engine — o motor (≙ Sources/CCUsageCore/Engine)

Modelos, formato da amostra, leitor de uso, credencial, rotação e o store. Sem UI, sem rede.

## Arquivos
- `provider.rs` — `Provider` (só `.anthropic` na v1), o trait `ProviderAdapter` (≙ protocolo do
  Swift: `credential_location`, `identity`, `write_identity`, `launch_command`) e `IdentityError`.
- `config_dir.rs` — `ConfigDir {raw, isDefault}`: o perfil; assimetria do `.claude.json`
  (ao lado do padrão, dentro do dedicado) e o valor de ambiente (`None` no padrão).
- `account_model.rs` — `AccountIdentity` (email, org, tier, `oauthAccount` cru) e `Account`.
- `group_model.rs` — `AccountGroup` e `RouterConfig` (o `config.json`). serde camelCase,
  `accountIDs` renomeado à mão, ids MAIÚSCULOS.
- `group_usage.rs` — `UsageOrigin`, `GroupUsageSample`, `ModelUsage`, `GroupUsageStore`.
  A escrita preserva o bloco por modelo quando a amostra nova não o traz.
- `group_usage_reader.rs` — `GroupUsageReader`/`AccountUsage`: maior janela válida, empate
  vai para a de horizonte mais longo (`max_by` = último dos empatados, igual `max(by:<=)`).
- `anthropic_adapter.rs` — `AnthropicAdapter`: lê o `.claude.json` (`identity`), grava a
  identidade de forma **cirúrgica** (`write_identity` + `splice_identity`) e diz onde mora a
  credencial (`<perfil>\.credentials.json`).
- `credential_store.rs` — ≙ `KeychainStore`: `CredentialBlob` (bytes OPACOS; `Debug` não mostra
  o conteúdo; `is_complete` = checagem estrutural "JSON completo com objeto `claudeAiOauth`", que
  pula os valores sem guardá-los), o trait `CredentialStore` e o `FileCredentialStore`
  (temp+rename com mtime novo, bytes idênticos não regravados, blob incompleto recusado na
  escrita e não devolvido na leitura, que espera ~0,2 s por uma escrita em andamento).
- `default_profile_guard.rs` — antes da 1ª escrita do router no perfil padrão (`~\.claude`), copia o
  login que havia (`.credentials.json` + `oauthAccount`) para `<base>\backups\default-profile-<ts>\`.
- `rotation_engine.rs` — ≙ `RotationEngine`, regra por regra: conta ativa pela identidade do
  perfil; `activate` (reativar só espelha grupo→casa; recusa conta ativa noutro grupo; espelha a
  que sai; copia casa→grupo e grava a identidade); `mirror_group_to_home` (só se mudou);
  `push_home_to_group` (só pós-relogin); `mirror_active`; `probe_config_dir` (ativa → perfil do
  grupo, nunca a casa); `next_account`/`rotation_target` (sem amostra = fresca; limiar estrito).
- `account_login_service.rs` — `login_result`: identidade no `.claude.json` **e** credencial
  completa na casa; uma sem a outra é login pela metade.
- `router_config_store.rs` — ≙ `RouterConfigStore` (sem UI): grupos, contas, login/relogin,
  remoção, ativar, `refresh_usage` (uso, ativa e sessões vivas por grupo — leitor de sessões
  injetável), `rotate_all`. Erros tipados em `StoreError`.
- `engine_lock.rs` — trava entre processos (mutex nomeado `Local\com.synqo.falcao-router.engine.<fnv>`,
  nome derivado da base) em volta de toda escrita de credencial do store e da CLI.
- `session_launcher.rs` — ≙ `SessionLauncher`: `group_named` (sem caixa, sem espaço nas pontas) e
  `prepare` (ativa com folga → próxima com folga → ativa → primeira; erro de ativação vira
  `NoUsableAccount`). O processo fica na CLI.
- `provider_env.rs` — ≙ `ProviderEnv`, **sem caixa**: listas do macOS + `WINDOWS_KEYS`/`WINDOWS_PREFIXES`
  (token OAuth por variável/arquivo/descritor, `CLAUDE_SECURESTORAGE_CONFIG_DIR`, perfil/org
  alternativos, identidade federada, Foundry, Bedrock por token); `without_nested_session`
  (`CLAUDE_CODE*`, `CLAUDECODE`) para sonda e login; `with_var`.
- `session_registry.rs` — ≙ `SessionRegistry`/`ProcessLiveness`: lê `<perfil>\sessions\*.json`
  (exige `pid` e `cwd`), `procStart` FILETIME (Windows) ou `ctime` (macOS), filtra `pidDomain`
  de outra máquina, confere o processo (tolerância 300 s; sem prova confia no pid), mais nova
  primeiro.
- `router_paths.rs` — base `%LOCALAPPDATA%\com.synqo.falcao-router` (override `ROUTER_APP_SUPPORT`).

## Decisões
- 22/09/2026: portado 1:1 do Swift. Datas ISO-8601 **sem fração**; `origin` ausente = sensor.
- 22/09/2026: `write_identity` é cirúrgico — troca só `oauthAccount`, põe
  `hasCompletedOnboarding: true`, tira `cachedUsageUtilization` (`shift_remove`: o `remove` do
  `serde_json` com `preserve_order` troca a última chave de lugar). Preserva ordem e chaves que só
  diferem em caixa; **recusa** arquivo existente e ilegível (o macOS o trocava por `{}`); arquivo
  vazio conta como ausente; relê depois de gravar. Saída com recuo de 2 (como o Claude Code).
- 22/09/2026: a credencial é arquivo; `ProviderAdapter::credential_location` (≙ `keychainService`)
  devolve `<perfil>\.credentials.json` nos dois tipos de perfil. Os 4 testes de hash do macOS não
  se aplicam; no lugar, testes de onde a credencial mora.
- 22/09/2026: o store diverge do macOS de propósito: home injetada (os testes do macOS usavam a
  real); erro tipado (texto só na UI); `config.json` ilegível guardado de lado
  (`config.unreadable-<ts>.json`) no 1º `save`, nunca sobrescrito; reordenar não tira do grupo a
  conta que a lista esqueceu; remover conta só apaga pasta sob `<base>\accounts\`.
- 22/09/2026: trava do motor é segurança a mais, não portão — sem ela no prazo (5 s), segue.
- O item do GRUPO não é apagado ao remover conta nem grupo (paridade macOS): pode haver sessão viva.

## Pendências (Fase 4)
- `profile_sharing`, `shell_integration`; no store: `measure_accounts` e a integração de terminal.
