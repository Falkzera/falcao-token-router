import Foundation

/// Um alerta a emitir. **Sem texto**: carrega o fato, não a frase.
///
/// Quem redige é o presenter, no alvo de UI, onde já moram todas as strings de
/// usuário. Copy em português aqui obrigaria a localizar o core junto quando os
/// idiomas chegarem — e faria os testes afirmarem sobre redação em vez de sobre
/// comportamento.
public enum Alert: Equatable, Sendable {
    /// A fração cruzou `percent` para cima, numa janela que reseta em `resetsAt`.
    case threshold(window: Window, percent: Int, resetsAt: Date?)
    /// A janela resetou — e o usuário tinha encostado no teto antes disso.
    case windowReset(window: Window)
    /// Alertas ligados sem busca ao vivo: nada pode ser afirmado com confiança.
    case liveRequired

    public enum Window: String, Codable, Equatable, Sendable, CaseIterable {
        case session
        case weekly
    }
}
