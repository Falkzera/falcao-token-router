import AppKit
import SwiftUI
import CCUsageCore

/// O anel-medidor: a marca do app.
///
/// A geometria mora em `GaugeGeometry`, no CCUsageCore, e não aqui. Não é só
/// por testabilidade: o gerador do ícone (`Scripts/icon.swift`) compila aquele
/// mesmo arquivo, então o anel do Finder e o da barra de menu saem da mesma
/// construção em vez de serem dois desenhos que alguém precisa lembrar de
/// manter parecidos.
struct GaugeMark: View {
    let fraction: Double
    var lineWidth: CGFloat = 2

    var body: some View {
        GeometryReader { geometry in
            let rect = CGRect(origin: .zero, size: geometry.size)
            ZStack {
                // A trilha é o próprio anel a 100%, esmaecido: ela mostra o que
                // ainda cabe. Como imagem template o sistema tinta pelo alfa,
                // então esta opacidade vira um tom mais claro da mesma cor —
                // é assim que dois níveis cabem num desenho de uma cor só.
                Path(GaugeGeometry.trackPath(in: rect, lineWidth: lineWidth))
                    .stroke(style: StrokeStyle(lineWidth: lineWidth))
                    .opacity(0.3)
                Path(GaugeGeometry.arcPath(in: rect, lineWidth: lineWidth, fraction: fraction))
                    .stroke(style: StrokeStyle(lineWidth: lineWidth, lineCap: .round))
            }
        }
    }
}

extension GaugeMark {
    /// Quantos desenhos distintos o anel assume na barra de menu.
    static let menuBarSteps = 20
    private static let menuBarSize: CGFloat = 13

    @MainActor private static var cache: [Int: NSImage] = [:]

    /// O label do `MenuBarExtra` só renderiza `Text` e `Image` — uma `Shape`
    /// desenhada direto no `HStack` é silenciosamente descartada. Daí
    /// rasterizar. O cache existe porque, ao contrário do mark fixo que havia
    /// antes, este desenho muda com o consumo: sem ele seria um render a cada
    /// atualização, com ele são no máximo 21 na vida do processo.
    @MainActor
    static func menuBarImage(fraction: Double) -> NSImage {
        let step = GaugeGeometry.quantize(fraction, steps: menuBarSteps)
        if let cached = cache[step] { return cached }

        // Desenha a fração já quantizada, não a original: o bitmap guardado sob
        // uma chave precisa ser o desenho daquela chave.
        let renderer = ImageRenderer(
            content: GaugeMark(fraction: Double(step) / Double(menuBarSteps))
                .frame(width: menuBarSize, height: menuBarSize)
                .foregroundStyle(.black))
        renderer.scale = 4

        let image = renderer.nsImage ?? NSImage(size: NSSize(width: menuBarSize, height: menuBarSize))
        image.isTemplate = true
        cache[step] = image
        return image
    }
}
