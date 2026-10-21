# Settings — agent.md

## Propósito
As preferências do usuário e a detecção do plano. Fica no core, e não na UI, para ser exercitável sem janela — mesma regra do resto do módulo.

## Arquivos
- `AppSettings.swift` — `Plan` (assinatura e preço, que é dado de negócio) e o objeto observável persistido em `UserDefaults`. Também a **migração do domínio antigo**, ver abaixo.
- `PlanDetector.swift` — lê o plano do item de chaveiro que o Claude Code mantém. **Lê exclusivamente `rateLimitTier`**; `ClaudeCredentials` não tem campo para `refreshToken`, e isso é o mecanismo, não esquecimento.

## Padrões
- Escolha explícita do usuário vence a detecção, que vence o padrão de fábrica. Divergência entre o detectado e o escolhido é **informação**, e a UI avisa em vez de trocar por baixo dele.
- Payload corrompido não pode impedir o app de abrir.

## Decisões recentes
- 2026-09-22: `migrateLegacyDefaults`. `UserDefaults.standard` é indexado pelo `CFBundleIdentifier`; renomear o bundle apontou o app para um domínio vazio e o plano, os alertas e o teto do usuário ficaram no antigo, invisíveis, sem nada avisando. A migração roda **só no domínio real do app** — puxar um domínio externo fixo para dentro de um `UserDefaults` de teste contamina o chamador com a configuração da máquina (dois testes quebraram assim).

- 2026-10-21: `scheduledMeasure` — "Medir as contas a cada 5 horas", para a aba Histórico. **Ligada de fábrica**, e um payload de antes dela (sem o campo) também resolve para ligada: o histórico só existe se a medição rodar, e quem atualiza o app não deveria ter de descobrir uma chave para ver a aba nova funcionar.

## Pendências conhecidas
- `PlanDetector` é o único leitor do produto que usa `SecItemCopyMatching` em vez do `/usr/bin/security` que o resto usa para não disparar o prompt do macOS. A causa real é a *partition list* do item de chaveiro, que nenhuma GUI escreve.
