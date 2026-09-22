# engine — o motor (≙ Sources/CCUsageCore/Engine)

Modelos, formato da amostra, leitor de uso, adapter e caminhos. Sem UI, sem rede.

## Arquivos (nesta fatia, o sensor)
- `provider.rs` — `Provider` (só `.anthropic` na v1), o trait `ProviderAdapter` (≙ protocolo do
  Swift: `identity`, `write_identity`, `launch_command`) e `IdentityError`.
- `config_dir.rs` — `ConfigDir {raw, isDefault}`: o perfil; assimetria do `.claude.json`
  (ao lado do padrão, dentro do dedicado) e o valor de ambiente (`None` no padrão).
- `account_model.rs` — `AccountIdentity` (email, org, tier, `oauthAccount` cru) e `Account`.
- `group_model.rs` — `AccountGroup` e `RouterConfig` (o `config.json`). serde camelCase,
  `accountIDs` renomeado à mão, ids MAIÚSCULOS.
- `group_usage.rs` — `UsageOrigin`, `GroupUsageSample`, `ModelUsage`, `GroupUsageStore`.
  A escrita preserva o bloco por modelo quando a amostra nova não o traz.
- `group_usage_reader.rs` — `GroupUsageReader`/`AccountUsage`: maior janela válida, empate
  vai para a de horizonte mais longo (`max_by` = último dos empatados, igual `max(by:<=)`).
- `anthropic_adapter.rs` — `AnthropicAdapter`: lê o `.claude.json` (`identity`) e grava a
  identidade de forma **cirúrgica** (`write_identity` + `splice_identity`).
- `router_paths.rs` — base `%LOCALAPPDATA%\com.synqo.falcao-router` (override `ROUTER_APP_SUPPORT`).

## Decisões
- 22/09/2026: portado 1:1 do Swift. Datas ISO-8601 **sem fração**; `origin` ausente = sensor.
- 22/09/2026: `write_identity` é cirúrgico — troca só `oauthAccount`, põe
  `hasCompletedOnboarding: true`, tira `cachedUsageUtilization` (`shift_remove`: o `remove` do
  `serde_json` com `preserve_order` troca a última chave de lugar). Preserva ordem e chaves que só
  diferem em caixa; **recusa** arquivo existente e ilegível (o macOS o trocava por `{}`); arquivo
  vazio conta como ausente; relê depois de gravar. Saída com recuo de 2 (como o Claude Code).
- No Windows não há hash de chaveiro; o `config.json` mantém `configDir {raw,isDefault}` igual.

## Pendências (Fase 4)
- Credencial em arquivo, `RotationEngine`,
  `RouterConfigStore`, `provider_env`, `session_registry`, liveness, resolvedor do `claude`.
