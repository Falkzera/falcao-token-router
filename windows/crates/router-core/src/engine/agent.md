# engine — o motor (≙ Sources/CCUsageCore/Engine)

Modelos, formato da amostra, leitor de uso, adapter e caminhos. Sem UI, sem rede.

## Arquivos (nesta fatia, o sensor)
- `provider.rs` — `Provider` (só `.anthropic` na v1).
- `config_dir.rs` — `ConfigDir {raw, isDefault}`: o perfil; assimetria do `.claude.json`
  (ao lado do padrão, dentro do dedicado) e o valor de ambiente (`None` no padrão).
- `account_model.rs` — `AccountIdentity` (email, org, tier, `oauthAccount` cru) e `Account`.
- `group_model.rs` — `AccountGroup` e `RouterConfig` (o `config.json`). serde camelCase,
  `accountIDs` renomeado à mão, ids MAIÚSCULOS.
- `group_usage.rs` — `UsageOrigin`, `GroupUsageSample`, `ModelUsage`, `GroupUsageStore`.
  A escrita preserva o bloco por modelo quando a amostra nova não o traz.
- `group_usage_reader.rs` — `GroupUsageReader`/`AccountUsage`: maior janela válida, empate
  vai para a de horizonte mais longo (`max_by` = último dos empatados, igual `max(by:<=)`).
- `anthropic_adapter.rs` — `identity`/`identity_from_bytes` (lê o `.claude.json`).
- `router_paths.rs` — base `%LOCALAPPDATA%\com.synqo.falcao-router` (override `ROUTER_APP_SUPPORT`).

## Decisões
- 22/09/2026: portado 1:1 do Swift. Datas ISO-8601 **sem fração**; `origin` ausente = sensor.
- No Windows não há hash de chaveiro; o `config.json` mantém `configDir {raw,isDefault}` igual.

## Pendências (Fase 4)
- `writeIdentity` (escrita cirúrgica do `.claude.json`), credencial em arquivo, `RotationEngine`,
  `RouterConfigStore`, `provider_env`, `session_registry`, liveness, resolvedor do `claude`.
