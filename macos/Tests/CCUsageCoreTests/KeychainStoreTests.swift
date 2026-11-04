import Foundation
import Testing
@testable import CCUsageCore

/// O único teste que toca o chaveiro de verdade — de propósito, porque o bug
/// mora no `/usr/bin/security`, não no nosso código. Item descartável: conta e
/// serviço que nunca coincidem com um `Claude Code-credentials-*`, então não há
/// credencial de conta em jogo. Fora da CI, que pode não ter chaveiro de login.
@Suite(.enabled(if: ProcessInfo.processInfo.environment["CI"] == nil))
struct SecurityCLIKeychainTests {
    let keychain = SecurityCLIKeychain(account: "falcao-router-teste")
    let service = "falcao-router-teste-\(UUID().uuidString)"

    /// Regressão de 04/11/2026: acima de ~4 KB o `security -i` gravava o
    /// segredo truncado e só então falhava.
    @Test(arguments: [100, SecurityCLIKeychain.stdinLineLimit, 6000])
    func segredoVoltaInteiro(bytes: Int) throws {
        defer { keychain.delete(service: service) }
        let secret = String(repeating: "x", count: bytes)
        try keychain.write(secret, service: service)
        #expect(keychain.read(service: service) == secret)
    }
}
