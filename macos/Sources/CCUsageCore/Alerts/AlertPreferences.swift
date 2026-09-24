import Foundation

/// O que o usuário quer ser avisado.
///
/// Um tipo em vez de uma lista de booleanos na assinatura de `evaluate`: com
/// cinco parâmetros do mesmo tipo, trocar dois de lugar compila e muda o
/// comportamento em silêncio.
public struct AlertPreferences: Codable, Equatable, Sendable {
    /// Avisos ao cruzar limiar de consumo.
    public var thresholdsEnabled: Bool
    /// Quais limiares, em pontos percentuais.
    public var thresholds: Set<Int>
    /// Em quais janelas — tanto para limiar quanto para reset.
    public var windows: Set<Alert.Window>
    /// Aviso de que a janela resetou e a capacidade voltou.
    public var resetEnabled: Bool

    public init(thresholdsEnabled: Bool, thresholds: Set<Int>,
                windows: Set<Alert.Window>, resetEnabled: Bool) {
        self.thresholdsEnabled = thresholdsEnabled
        self.thresholds = thresholds
        self.windows = windows
        self.resetEnabled = resetEnabled
    }

    /// Nada ligado — o estado de quem nunca abriu os Ajustes.
    public static let off = AlertPreferences(
        thresholdsEnabled: false, thresholds: [80, 95],
        windows: [.session, .weekly], resetEnabled: false)

    public static let `default` = AlertPreferences(
        thresholdsEnabled: true, thresholds: [80, 95],
        windows: [.session, .weekly], resetEnabled: true)

    /// Os limiares oferecidos na tela. 90 não vem ligado por padrão: entre 80 e
    /// 95 ele acrescenta um aviso a mais sem acrescentar decisão nova.
    public static let offeredThresholds = [80, 90, 95]

    public var anyEnabled: Bool { thresholdsEnabled || resetEnabled }
}
