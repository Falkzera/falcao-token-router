import Foundation

/// A medição automática de 5 em 5 horas (21/10/2026): QUANDO cada conta deve
/// ser medida de novo. Pura — o relógio entra por parâmetro.
///
/// A sonda era "botão e comando, nunca laço": cada medição sobe um Node e faz
/// uma requisição por conta. O histórico pediu uma medição a cada 5 horas —
/// umas 5 por conta por dia —, com o app aberto e uma chave nos Ajustes que a
/// desliga. Mesma regra do porte Windows (`measure_schedule.rs`).
public enum MeasureSchedule {
    /// O intervalo entre duas medições da mesma conta.
    public static let every: TimeInterval = 5 * 3600

    /// Quando a conta deve ser medida de novo: 5 horas depois da mais nova
    /// entre a última medição gravada (venha de onde vier) e a última tentativa
    /// da agenda nesta execução do app — sucesso ou falha. A tentativa é o que
    /// segura uma conta deslogada, que sem ela seria sondada a cada volta do
    /// laço. `nil`: nunca medida nem tentada.
    public static func nextAt(lastMeasured: Date?, lastAttempt: Date?) -> Date? {
        [lastMeasured, lastAttempt].compactMap { $0 }.max().map { $0.addingTimeInterval(every) }
    }

    /// A conta vence quando nunca foi medida nem tentada, ou quando a
    /// referência tem 5 horas ou mais.
    public static func isDue(lastMeasured: Date?, lastAttempt: Date?, now: Date) -> Bool {
        nextAt(lastMeasured: lastMeasured, lastAttempt: lastAttempt).map { $0 <= now } ?? true
    }
}
