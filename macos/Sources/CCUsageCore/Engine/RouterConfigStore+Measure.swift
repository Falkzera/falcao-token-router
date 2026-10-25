import Foundation

/// "Medir contas": a sonda ativa sob demanda, conta por conta, com a marca de
/// quem está sem login. A agenda de 5 em 5 horas mora no
/// `RouterConfigStore+History.swift`. Arquivo à parte pela régua de 600 linhas.
extension RouterConfigStore {
    /// Mede as contas de um grupo com a **sonda ativa**, e publica o resultado.
    ///
    /// É o que o sensor passivo não consegue: conta ociosa nunca serviu
    /// mensagem nenhuma, então não tem amostra — aparecia como "pronta", sem
    /// número, e a rotação a presumia fresca. A sonda pergunta ao binário
    /// oficial e traz o quadro inteiro, inclusive o limite POR MODELO, que não
    /// chega no `rate_limits` da status line e é o que estoura primeiro.
    ///
    /// Sob demanda (este botão) — e, com o app aberto, de 5 em 5 horas pela
    /// agenda (`runScheduledMeasure`, 21/10/2026). Nunca no laço de 3 minutos:
    /// cada conta custa um processo Node subindo do zero e uma requisição de
    /// verdade. Medir cinco contas leva ~15s. Toda medição deixa um ponto no
    /// histórico (`ProbeRecord`).
    ///
    /// `probe` é injetável para teste; sem ele, a sonda sobre o binário instalado.
    public func measureAccounts(in group: AccountGroup, using probe: ClaudeUsageProbe? = nil) async {
        guard measuringGroup == nil else { return }
        measuringGroup = group.id
        defer { measuringGroup = nil }

        let contas = config.accounts(in: group)
        let configNoClique = config
        let engine = self.engine
        let usageDir = paths.usageDir
        let historyDir = paths.historyDir
        let scratchBase = paths.base

        // Conta por conta, e não só quantas: o aviso que dizia "N conta(s) não
        // responderam" não deixava saber QUAL relogar.
        let saldo = await Task.detached(priority: .userInitiated) {
            () -> (responderam: [UUID], semLogin: [UUID], falhas: Int)? in
            guard let sonda = probe ?? ClaudeUsageProbe.system(scratchBase: scratchBase)
            else { return nil }
            var responderam: [UUID] = [], semLogin: [UUID] = [], falhas = 0
            for conta in contas {
                // O perfil é decidido AGORA, conta a conta, relendo do disco
                // quem está ativa: conta ativa vai pelo perfil do grupo, nunca
                // pela casa (ver `probeConfigDir` — sondar a casa de uma conta
                // ativa derruba a sessão viva). Decidido no clique, um
                // `claude <grupo>` aberto em outro terminal no meio da medição
                // fazia a sonda chegar na casa de uma conta que já estava ativa.
                // Dentro do app, a rotação e o "Usar" esperam a medição.
                let alvo = (email: conta.identity.email,
                            dir: engine.probeConfigDir(for: conta, config: configNoClique))
                do {
                    let leitura = try sonda.read(configDir: alvo.dir)
                    try ProbeRecord.write(leitura, email: alvo.email, configDir: alvo.dir,
                                          usageDir: usageDir, historyDir: historyDir)
                    responderam.append(conta.id)
                } catch ClaudeUsageProbe.ProbeError.notSignedIn {
                    falhas += 1
                    semLogin.append(conta.id)
                } catch {
                    // Prazo estourado, formato novo: falha, mas não diz nada
                    // sobre o login — a marca da conta fica como estava.
                    falhas += 1
                }
            }
            return (responderam: responderam, semLogin: semLogin, falhas: falhas)
        }.value

        // Texto cru, como o resto dos `lastError` deste arquivo: o core não
        // carrega catálogo (a CLI `router` o linka e não tem bundle de app).
        guard let saldo else {
            lastError = "binário `claude` não encontrado — a sonda precisa dele"
            refreshUsage()
            return
        }
        lastError = saldo.falhas == 0 ? nil
            : "\(saldo.falhas) conta(s) não responderam à sonda — veja se precisam de Relogar"
        for id in saldo.responderam { signedOut[id] = nil }
        let agora = Date()
        for id in saldo.semLogin { signedOut[id] = agora }
        refreshUsage()
    }
}
