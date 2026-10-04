# Settings — agent.md

## Propósito
As preferências do usuário e a detecção do plano. Fica no core, e não na UI, para ser exercitável sem janela — mesma regra do resto do módulo.

## Arquivos
- `AppSettings.swift` — `Plan` (assinatura e preço, que é dado de negócio) e o objeto observável persistido em `UserDefaults`. Também a **migração do domínio antigo**, ver abaixo.
- `PlanDetector.swift` — lê o plano do `oauthAccount` do `~/.claude.json` (`userRateLimitTier`, com o de organização como reserva). **Não toca o chaveiro.**

## Padrões
- Escolha explícita do usuário vence a detecção, que vence o padrão de fábrica. Divergência entre o detectado e o escolhido é **informação**, e a UI avisa em vez de trocar por baixo dele.
- Payload corrompido não pode impedir o app de abrir.

## Decisões recentes
- 2026-10-04: o plano passou a vir do `.claude.json`, não do chaveiro. O `PlanDetector` decodificava o item `Claude Code-credentials` inteiro — tokens incluídos — via `SecItemCopyMatching` a cada abertura do app, só para ler um campo, e a primeira leitura abria o prompt do macOS. O `oauthAccount` traz o mesmo tier sem segredo ao lado. `userRateLimitTier` vem primeiro porque, numa conta Max, o de organização não nomeava plano nenhum.
- 2026-09-22: `migrateLegacyDefaults`. `UserDefaults.standard` é indexado pelo `CFBundleIdentifier`; renomear o bundle apontou o app para um domínio vazio e o plano, os alertas e o teto do usuário ficaram no antigo, invisíveis, sem nada avisando. A migração roda **só no domínio real do app** — puxar um domínio externo fixo para dentro de um `UserDefaults` de teste contamina o chamador com a configuração da máquina (dois testes quebraram assim).

## Pendências conhecidas
- Nenhuma.
