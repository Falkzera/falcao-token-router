import SwiftUI

/// Converte a linha que o `router` imprime — com os escapes ANSI e tudo — numa
/// `AttributedString` que o SwiftUI desenha.
///
/// Existe por um motivo de correção, não de enfeite: a prévia dos Ajustes é
/// desenhada pelo **mesmo código** que desenha a sessão (`StatusLineView.render`
/// com a mesma `StatusLineChoice`). Se a prévia remontasse a linha com views
/// próprias, ela poderia discordar do que a sessão mostra — e a prévia existe
/// justamente para o usuário confiar no que vai ver.
///
/// Entende o subconjunto que a linha usa: reset (`0`), negrito (`1`), as cores
/// 30–37/90–97 e a truecolor `38;2;r;g;b`. Qualquer outro código é ignorado sem
/// aparecer como texto.
enum ANSIText {
    static func attributed(_ raw: String) -> AttributedString {
        var out = AttributedString()
        var color: Color?
        var bold = false
        var text = ""

        func flush() {
            guard !text.isEmpty else { return }
            var piece = AttributedString(text)
            piece.foregroundColor = color ?? .primary
            if bold { piece.inlinePresentationIntent = .stronglyEmphasized }
            out += piece
            text = ""
        }

        var rest = Substring(raw)
        while let esc = rest.firstIndex(of: "\u{1b}") {
            text += rest[rest.startIndex..<esc]
            guard let end = rest[esc...].firstIndex(of: "m") else {
                rest = rest[rest.index(after: esc)...]
                continue
            }
            flush()
            let codes = rest[rest.index(esc, offsetBy: 2)..<end].split(separator: ";").map(String.init)
            apply(codes, color: &color, bold: &bold)
            rest = rest[rest.index(after: end)...]
        }
        text += rest
        flush()
        return out
    }

    private static func apply(_ codes: [String], color: inout Color?, bold: inout Bool) {
        var i = 0
        while i < codes.count {
            switch codes[i] {
            case "0", "":
                color = nil; bold = false
            case "1":
                bold = true
            case "38":
                // `38;2;r;g;b` — truecolor. Qualquer outra forma de 38 é pulada.
                guard i + 4 < codes.count, codes[i + 1] == "2",
                      let r = Double(codes[i + 2]), let g = Double(codes[i + 3]),
                      let b = Double(codes[i + 4])
                else { i = codes.count; break }
                color = Color(red: r / 255, green: g / 255, blue: b / 255)
                i += 4
            case let code:
                if let named = named(code) { color = named }
            }
            i += 1
        }
    }

    /// As 8 cores e suas versões claras, nos tons do terminal escuro em que a
    /// linha é lida — não nas cores do sistema, que mudariam com o tema.
    private static func named(_ code: String) -> Color? {
        switch code {
        case "30", "90": Color(red: 0.60, green: 0.60, blue: 0.60)
        case "31", "91": Color(red: 1.00, green: 0.42, blue: 0.40)
        case "32", "92": Color(red: 0.42, green: 0.84, blue: 0.47)
        case "33", "93": Color(red: 0.96, green: 0.76, blue: 0.29)
        case "34":       Color(red: 0.36, green: 0.60, blue: 0.98)
        case "94":       Color(red: 0.69, green: 0.73, blue: 0.98)
        case "35", "95": Color(red: 0.80, green: 0.55, blue: 0.95)
        case "36", "96": Color(red: 0.36, green: 0.82, blue: 0.87)
        case "37", "97": Color(red: 0.90, green: 0.90, blue: 0.90)
        default: nil
        }
    }
}
