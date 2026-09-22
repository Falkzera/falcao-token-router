import Foundation

/// Por que a fonte oficial de uso não pôde ser lida. Cada caso vira uma frase
/// diferente na UI, porque a saída para o usuário é diferente: reautenticar,
/// esperar, ou nada a fazer.
///
/// A fonte oficial passou a ser o **sensor** (o `rate_limits` que o Claude Code
/// entrega na status line), não mais uma chamada de API própria — então na
/// prática só `noToken` (nenhuma amostra recente) acontece. Os demais casos
/// ficam porque a política de origem ainda os distingue e um provedor futuro
/// pode reintroduzi-los.
public enum LiveUsageError: Error, Equatable {
    /// Sem amostra recente do sensor para a conta ativa. Nenhuma leitura de rede.
    case noToken
    /// Credencial recusada (histórico; o sensor não autentica nada).
    case unauthorized
    /// Fonte indisponível.
    case transport
    /// Resposta que não dá para interpretar.
    case malformed
}
