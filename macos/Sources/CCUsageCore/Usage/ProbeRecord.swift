import Foundation

/// O que uma medição da sonda grava (21/10/2026): a amostra — o número que
/// decide a rotação — e o ponto do histórico.
///
/// Num lugar só porque três caminhos medem (o "Medir contas", a agenda de 5 em
/// 5 horas e o `router measure`), e antes o store e a CLI montavam a amostra à
/// mão, cada um a sua cópia.
public enum ProbeRecord {
    /// Grava a amostra (origem `probe`, com as janelas por modelo e o carimbo
    /// delas) e depois o ponto do histórico. Falha no histórico não desfaz a
    /// medição: a amostra já está no disco, e é ela que a rotação lê.
    public static func write(_ reading: ClaudeUsageProbe.Reading, email: String,
                             configDir: ConfigDir, usageDir: URL, historyDir: URL,
                             now: Date = Date()) throws {
        let sample = GroupUsageSample(
            configDirRaw: configDir.raw, email: email,
            fiveHourPercent: reading.session, fiveHourResetsAt: reading.sessionResetsAt,
            sevenDayPercent: reading.weeklyAll, sevenDayResetsAt: reading.weeklyAllResetsAt,
            sampledAt: now,
            models: ModelUsage(windows: reading.models, sampledAt: now),
            origin: .probe)
        try GroupUsageStore.write(sample, forEmail: email, in: usageDir)
        try? UsageHistory.append(UsageHistoryPoint(reading: reading, sampledAt: now),
                                 forEmail: email, in: historyDir, now: now)
    }
}
