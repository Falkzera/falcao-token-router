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

- `shell_integration.rs` — ≙ `ShellIntegration`: a `statusLine` (`/` e sem aspas; com espaço, 8.3;
  sem 8.3, aspas do shell detectado — `&` no PowerShell; `--profile` nos grupos dedicados),
  `install_status_line` cirúrgico, obsolescência por igualdade exata; `shell.ps1` (UTF-8 com BOM,
  CRLF) e `shell.sh` (sem BOM, LF), ambos sem `exec` e **encadeando** uma função `claude` que já
  existia no perfil (marcador `falcao-router-shim` — no bash como comando `:`, porque o
  `declare -f` joga fora comentários); linhas de perfil ASCII guardadas por `Test-Path`/`[ -f ]`;
  `ShellTargets` (os dois `$PROFILE` sob a Documentos real, `.bashrc`); `ensure_bash_profile`.
- `terminal_report.rs` — o estado da integração POR SHELL (novo, fase 5): edições do PowerShell
  (5.1 do sistema; `pwsh` no `ProgramFiles` ou no PATH), política efetiva sem o escopo Process e
  a correção consentida (`RemoteSigned` em CurrentUser), `function claude` do usuário (UTF-8 e
  UTF-16LE), perfil de login do Git Bash, scripts atuais/obsoletos, e o `TerminalReport` que a
  tela de Grupos mostra e o `doctor` confere.
- `profile_sharing.rs` — ≙ `ProfileSharing`: pastas por junction; arquivos por symlink quando o
  Windows deixa, senão plano B (`CLAUDE.md`/`keybindings.json` copiados com mtime da origem e
  sincronizados — mais novo vence, backup do sobrescrito, só o que está no manifesto
  `.falcao-router-sync.json`; `history.jsonl` fica por grupo). Nunca troca item real do grupo.

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

- 22/09/2026: a função `claude` do router ENCADEIA a que já existia no perfil (nesta máquina o
  perfil do PowerShell define uma que escolhe a conta do `claude` puro) — sobrescrevê-la mudaria
  em silêncio a conta do `claude` sem grupo. `claude <grupo>` vai para o router; o resto, para ela.
- 22/09/2026: a status line de grupo dedicado leva `--profile`; o sensor prefere o
  `CLAUDE_CONFIG_DIR` e cai no `--profile` quando a variável não chega (subprocesso raspado).

- 22/09/2026: `measure_accounts` do store é síncrono (o app o chama fora da thread da UI) e usa
  `probe_targets` — o perfil de cada conta decidido com o config na mão (ativa → grupo).

- 22/09/2026 (fase 5, o que o app pede ao store):
  - `add_group_with(nome, padrão?)`: o 1º grupo pode nascer DEDICADO. O `rotation_target` ativa a
    primeira conta de um grupo sem ativa, então um grupo padrão trocaria o login do `~\.claude`
    (com backup) em até 180 s depois da primeira conta — o app pergunta antes quando
    `foreign_default_login` acha lá um login que o router não conhece.
  - `exclusive_account_ids`: o número certo na confirmação de apagar grupo (o macOS contava a
    compartilhada, que não é apagada).
  - Medição em três passos (`measure_plan` → `MeasurePlan::run` → `finish_measure`): o app só
    segura o store para planejar e publicar; a sonda leva segundos por conta.
  - `discard_pending_home`: login cancelado ou duplicado apaga a casa reservada (pode ter
    credencial de verdade) — nunca a de conta registrada (relogin usa a mesma casa), nunca fora
    de `<base>\accounts\`.
