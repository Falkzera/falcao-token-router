import Foundation

/// Como a status line escreve cada pedaço de texto: o nome do modelo, os
/// tokens, o caminho encurtado e — o que mais importa fora dela — **quando uma
/// janela reseta** (≙ a parte de formatação de `view.rs` no porte Windows).
///
/// `resetWhen` é pública de propósito: a janela de Grupos escreve o reset com
/// ela, e é isso que garante que a tela e a sessão nunca discordem.
public enum StatusLineFormat {
    /// O "quando" de um reset, como a linha o escreve: a de 5 h só a hora
    /// (o reset cai nas próximas horas), a semanal com o dia, porque pode cair
    /// em qualquer um.
    ///
    ///     resetWhen(…, withDay: false) → "14:05"
    ///     resetWhen(…, withDay: true)  → "seg (28) 9:00"
    public static func resetWhen(_ date: Date, withDay: Bool,
                                 portuguese: Bool = Locale.current.language.languageCode?.identifier == "pt",
                                 calendar: Calendar = .current) -> String {
        let parts = calendar.dateComponents([.weekday, .day, .hour, .minute], from: date)
        let hour = parts.hour ?? 0, minute = parts.minute ?? 0
        guard withDay else { return String(format: "%02d:%02d", hour, minute) }

        // Hora sem zero à esquerda no formato semanal, de propósito — é o que o
        // porte Windows imprime, e as duas plataformas escrevem igual.
        let pt = ["dom", "seg", "ter", "qua", "qui", "sex", "sáb"]
        let en = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
        let names = portuguese ? pt : en
        let weekday = (parts.weekday ?? 1) - 1          // Calendar: 1 = domingo
        return "\(names[weekday % 7]) (\(parts.day ?? 0)) \(hour):\(String(format: "%02d", minute))"
    }

    /// "Opus 5 (1M context)" → "Opus 5": a janela já aparece no medidor de
    /// contexto, ao lado. Um parêntese aninhado não é rótulo de janela — fica.
    public static func modelName(_ display: String) -> String {
        let trimmed = display.trimmingCharacters(in: .whitespaces)
        guard trimmed.hasSuffix(")"),
              let open = trimmed.dropLast().lastIndex(of: "(")
        else { return trimmed }
        let inside = trimmed[trimmed.index(after: open)..<trimmed.index(before: trimmed.endIndex)]
        guard !inside.contains(")") else { return trimmed }
        return String(trimmed[trimmed.startIndex..<open]).trimmingCharacters(in: .whitespaces)
    }

    /// Fable em vermelho, para não passar batido — é o limite POR MODELO, o que
    /// costuma travar a conta antes das outras janelas. O resto em azul.
    static func modelColor(_ display: String, id: String?) -> String {
        "\(display) \(id ?? "")".lowercased().contains("fable") ? Paint.red : Paint.blue
    }

    static func tokens(_ n: UInt64) -> String {
        n >= 1_000 ? "\(UInt64((Double(n) / 1_000).rounded()))k" : "\(n)"
    }

    /// Fora de um repositório, a pasta: `~` no lugar da home, e as do meio viram
    /// `…` quando são muitas.
    public static func shortenPath(_ path: String, home: String) -> String {
        func normalize(_ p: String) -> String {
            var s = p
            while s.count > 1 && s.hasSuffix("/") { s.removeLast() }
            return s
        }
        var shown = normalize(path)
        let home = normalize(home)
        if !home.isEmpty, shown.lowercased().hasPrefix(home.lowercased()) {
            let rest = String(shown.dropFirst(home.count))
            if rest.isEmpty || rest.hasPrefix("/") { shown = "~" + rest }
        }
        let parts = shown.split(separator: "/", omittingEmptySubsequences: true)
        guard parts.count > 3 else { return shown }
        return [String(parts[0]), "…", String(parts[parts.count - 2]), String(parts[parts.count - 1])]
            .joined(separator: "/")
    }
}
