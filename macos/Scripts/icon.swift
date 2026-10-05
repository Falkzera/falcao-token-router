//
// Gera a arte do app: o .iconset, o fundo do DMG, o banner do README e o card
// social do GitHub.
//
// Desde 05/10/2026 a arte é da marca Falcão: símbolo e nome saem dos SVGs
// oficiais (repo falcao-identidade-visual), com o marinho #0B2D4C. O anel
// continua sendo o medidor, e é a barra de menus que o desenha: o anel mede, o
// falcão assina.
//
// Uso: icongen <diretório de saída> [captura-crua-do-painel.png]

import AppKit
import CoreGraphics
import CoreText
import Foundation
import ImageIO
import UniformTypeIdentifiers

// MARK: - Paleta

/// As cores da marca-mãe (identidade-visual.md, seção 4). Chapadas: a marca não
/// usa degradê.
enum Palette {
    static func hex(_ value: Int, alpha: CGFloat = 1) -> CGColor {
        CGColor(red: CGFloat((value >> 16) & 0xFF) / 255, green: CGFloat((value >> 8) & 0xFF) / 255,
                blue: CGFloat(value & 0xFF) / 255, alpha: alpha)
    }
    /// Marinho: a cor da marca. Placa do ícone e fundo do banner e do card.
    static let navy = hex(0x0B2D4C)
    /// Marinho mais claro: o símbolo ampliado ao fundo (splash, modelo 2 do manual).
    static let navyLight = hex(0x123A60)
    static let white = CGColor(gray: 1, alpha: 1)
    /// Nuvem: texto claro secundário sobre o marinho.
    static let cloud = hex(0xE9ECEE)
    /// Pena: 5,68:1 com o marinho. Nunca texto sobre fundo claro (2,47:1).
    static let feather = hex(0x9AA6B4)
    /// Névoa e Ardósia: o fundo claro do DMG e o texto secundário sobre ele (6,23:1).
    static let mist = hex(0xF4F5F7)
    static let slate = hex(0x4A5D70)
    static let shadow = CGColor(gray: 0, alpha: 0.28)
}

// MARK: - A marca

/// Os desenhos oficiais, copiados dos SVGs da marca (`logo/falcao-simbolo-*.svg`
/// e `logo/falcao-horizontal-*.svg`). **Não editar à mão**: a marca proíbe
/// redesenhar o símbolo. Se o desenho mudar lá, copie os `d` de novo.
enum BrandMark {
    /// `asa-corpo`, `barra-superior-cabeca` e `haste-barra-media`.
    static let wing = "M66 14C88 42 122 54 150 64C163 71 170 84 170 100C170 114 167 126 162 136C150 150 134 158 112 166C128 154 144 142 143 128C141 116 118 113 90 94C108 101 128 104 142 106C120 96 88 82 68 52C90 68 124 78 143 90C112 72 78 50 66 14Z"
    static let head = "M166 72C196 66 228 63 260 62C248 77 232 88 211.6 92C207.6 83.2 199.4 78.6 189.6 78.6C182.8 78.6 177.4 80 173.6 81.6C171.6 78 169 74.6 166 72Z"
    static let stem = "M152 156C149 172 142 188 131 203C149 189 161 169 177 155C194 141 213 129 236 118C216 122 202 125 190 128C176 133 163 144 152 156Z"
    /// O nome FALCÃO, já em curvas: não depende de fonte (e não se redigita).
    static let name = "M299.47 84.76Q299.34 85.41 298.88 85.87Q298.43 86.32 297.78 86.32L297.78 86.32L272.38 86.32Q271.86 86.32 271.60 86.98L271.60 86.98L269.64 103.39Q269.64 104.04 270.17 104.04L270.17 104.04L284.10 104.04Q284.75 104.04 285.14 104.49Q285.53 104.95 285.40 105.60L285.40 105.60L283.45 121.10Q283.45 121.75 282.93 122.21Q282.41 122.66 281.76 122.66L281.76 122.66L267.95 122.66Q267.43 122.66 267.17 123.31L267.17 123.31L263.00 157.44Q262.87 158.09 262.42 158.54Q261.96 159 261.31 159L261.31 159L243.33 159Q242.68 159 242.29 158.54Q241.90 158.09 242.03 157.44L242.03 157.44L252.84 69.39Q252.97 68.74 253.43 68.29Q253.88 67.83 254.54 67.83L254.54 67.83L299.99 67.83Q301.42 67.83 301.42 69.39L301.42 69.39L299.47 84.76ZM328.12 159Q326.69 159 326.69 157.57L326.69 157.57L326.43 146.24Q326.56 145.98 326.30 145.78Q326.04 145.58 325.78 145.58L325.78 145.58L312.10 145.58Q311.32 145.58 311.32 146.24L311.32 146.24L308.46 157.57Q308.07 159 306.63 159L306.63 159L288.66 159Q287.10 159 287.62 157.31L287.62 157.31L316.66 69.26Q317.05 67.83 318.49 67.83L318.49 67.83L339.06 67.83Q340.63 67.83 340.63 69.26L340.63 69.26L347.92 157.31L347.92 157.70Q347.92 159 346.49 159L346.49 159L328.12 159ZM316.27 128.26Q316.27 128.91 316.66 128.91L316.66 128.91L325.13 128.91Q325.78 128.91 325.78 128.26L325.78 128.26L324.87 98.57Q324.87 98.05 324.61 98.05Q324.35 98.05 324.09 98.57L324.09 98.57L316.27 128.26ZM355.60 159Q354.95 159 354.56 158.54Q354.17 158.09 354.30 157.44L354.30 157.44L365.11 69.39Q365.24 68.74 365.70 68.29Q366.15 67.83 366.81 67.83L366.81 67.83L384.78 67.83Q385.43 67.83 385.82 68.29Q386.21 68.74 386.08 69.39L386.08 69.39L377.49 139.85Q377.22 140.51 378.01 140.51L378.01 140.51L403.79 140.51Q404.45 140.51 404.84 140.96Q405.23 141.42 405.10 142.07L405.10 142.07L403.14 157.44Q403.14 158.09 402.62 158.54Q402.10 159 401.45 159L401.45 159L355.60 159ZM434.01 160.04Q423.07 160.04 416.82 154.05Q410.57 148.06 410.57 137.64L410.57 137.64Q410.57 136.21 410.83 133.34L410.83 133.34L415.78 93.36Q417.21 81.11 425.28 73.95Q433.36 66.79 445.34 66.79L445.34 66.79Q456.28 66.79 462.60 72.71Q468.92 78.64 468.92 88.93L468.92 88.93Q468.92 90.23 468.66 93.36L468.66 93.36L468.52 94.53Q468.39 95.18 467.94 95.64Q467.48 96.09 466.83 96.09L466.83 96.09L448.73 96.87Q447.30 96.87 447.43 95.31L447.43 95.31L447.95 91.79Q448.21 88.80 446.90 87.04Q445.60 85.28 443.13 85.28L443.13 85.28Q440.65 85.28 438.96 87.04Q437.27 88.80 436.88 91.79L436.88 91.79L431.54 135.17Q431.28 138.03 432.51 139.79Q433.75 141.55 436.22 141.55L436.22 141.55Q438.70 141.55 440.46 139.79Q442.22 138.03 442.61 135.17L442.61 135.17L443.00 131.39Q443.13 130.74 443.58 130.28Q444.04 129.83 444.69 129.83L444.69 129.83L462.53 130.61Q463.97 130.61 463.97 132.17L463.97 132.17L463.71 133.34Q462.27 145.45 454.13 152.75Q445.99 160.04 434.01 160.04L434.01 160.04ZM504.73 159Q503.30 159 503.30 157.57L503.30 157.57L503.04 146.24Q503.17 145.98 502.91 145.78Q502.65 145.58 502.39 145.58L502.39 145.58L488.71 145.58Q487.93 145.58 487.93 146.24L487.93 146.24L485.07 157.57Q484.67 159 483.24 159L483.24 159L465.27 159Q463.71 159 464.23 157.31L464.23 157.31L493.27 69.26Q493.66 67.83 495.09 67.83L495.09 67.83L515.67 67.83Q517.24 67.83 517.24 69.26L517.24 69.26L524.53 157.31L524.53 157.70Q524.53 159 523.10 159L523.10 159L504.73 159ZM492.88 128.26Q492.88 128.91 493.27 128.91L493.27 128.91L501.74 128.91Q502.39 128.91 502.39 128.26L502.39 128.26L501.48 98.57Q501.48 98.05 501.22 98.05Q500.96 98.05 500.69 98.57L500.69 98.57L492.88 128.26ZM515.02 59.89Q512.16 59.89 508.38 57.80L508.38 57.80Q507.47 57.28 506.56 56.63Q505.64 55.98 504.99 55.72Q504.34 55.46 503.43 55.46L503.43 55.46Q500.96 55.46 499.00 58.19L499.00 58.19Q498.48 58.84 497.89 59.04Q497.31 59.23 496.79 58.71L496.79 58.71L490.93 54.02Q490.01 53.24 490.54 51.94L490.54 51.94Q492.62 46.99 496.14 44.52Q499.65 42.04 503.43 42.04L503.43 42.04Q507.21 42.04 511.37 45.04L511.37 45.04Q511.77 45.30 512.61 45.95Q513.46 46.60 514.31 46.93Q515.15 47.25 516.06 47.25L516.06 47.25Q518.80 47.25 520.75 44.91L520.75 44.91Q521.66 43.47 522.84 44.26L522.84 44.26L528.57 48.81Q529.09 49.34 529.09 49.86L529.09 49.86Q529.09 50.64 528.31 51.81L528.31 51.81Q525.57 56.11 522.05 58.00Q518.54 59.89 515.02 59.89L515.02 59.89ZM553.57 160.04Q542.50 160.04 536.19 153.99Q529.87 147.93 529.87 137.38L529.87 137.38Q529.87 135.95 530.13 133.08L530.13 133.08L534.95 93.75Q536.51 81.38 544.65 74.08Q552.79 66.79 565.03 66.79L565.03 66.79Q576.11 66.79 582.55 72.91Q589.00 79.03 589.00 89.58L589.00 89.58Q589.00 90.88 588.74 93.75L588.74 93.75L583.79 133.08Q582.36 145.45 574.09 152.75Q565.82 160.04 553.57 160.04L553.57 160.04ZM555.92 141.55Q558.65 141.55 560.48 139.53Q562.30 137.51 562.82 133.99L562.82 133.99L567.77 92.84Q567.90 92.19 567.90 91.14L567.90 91.14Q567.90 88.41 566.60 86.85Q565.30 85.28 562.82 85.28L562.82 85.28Q560.09 85.28 558.26 87.37Q556.44 89.45 556.05 92.84L556.05 92.84L550.97 133.99L550.84 135.43Q550.84 138.29 552.14 139.92Q553.44 141.55 555.92 141.55L555.92 141.55Z"

    /// Caixas medidas nos SVGs: o símbolo no espaço dele, o nome no da assinatura horizontal.
    static let symbolBox = CGRect(x: 66, y: 14, width: 194, height: 189)
    static let nameBox = CGRect(x: 242, y: 42.04, width: 347, height: 118)

    static let symbol: CGPath = {
        let path = CGMutablePath()
        for d in [wing, head, stem] { path.addPath(svgPath(d)) }
        return path
    }()
    static let wordmark: CGPath = svgPath(name)
}

/// Lê o atributo `d` dos SVGs oficiais. Só os comandos absolutos que eles usam
/// (M, L, Q, C, Z): um desenho novo com outro comando para o gerador em vez de
/// sair torto.
func svgPath(_ d: String) -> CGPath {
    let path = CGMutablePath()
    let tokens = d.matches(of: #/[A-Za-z]|-?(?:\d+\.?\d*|\.\d+)/#).map { String($0.output) }
    var i = 0
    var command = ""
    func number() -> CGFloat {
        defer { i += 1 }
        return CGFloat(Double(tokens[i])!)
    }
    func point() -> CGPoint { CGPoint(x: number(), y: number()) }
    while i < tokens.count {
        if tokens[i].first!.isLetter { command = tokens[i]; i += 1 }
        switch command {
        case "M": path.move(to: point()); command = "L"
        case "L": path.addLine(to: point())
        case "Q": let c = point(); path.addQuadCurve(to: point(), control: c)
        case "C": let c1 = point(), c2 = point(); path.addCurve(to: point(), control1: c1, control2: c2)
        case "Z": path.closeSubpath(); command = ""
        default: fatalError("comando SVG que a marca não usava: \(command)")
        }
    }
    return path
}

/// Desenha um caminho da marca dentro de `target`, sem distorcer (a marca proíbe).
func draw(_ path: CGPath, box: CGRect, into target: CGRect, color: CGColor, in context: CGContext) {
    let s = min(target.width / box.width, target.height / box.height)
    var t = CGAffineTransform(translationX: target.midX - s * box.midX, y: target.midY - s * box.midY)
        .scaledBy(x: s, y: s)
    guard let placed = path.copy(using: &t) else { return }
    context.addPath(placed)
    context.setFillColor(color)
    context.fillPath()
}

// MARK: - Fontes da marca

/// Barlow Condensed Italic para o nome do app, Barlow para o texto (manual,
/// seção 5). Vêm de `Scripts/fonts/` (OFL): o sistema não as tem.
enum BrandFont {
    static func load(_ file: String, size: CGFloat) -> CTFont {
        let url = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .appendingPathComponent("fonts").appendingPathComponent(file)
        guard let descriptors = CTFontManagerCreateFontDescriptorsFromURL(url as CFURL) as? [CTFontDescriptor],
              let first = descriptors.first else { fatalError("fonte ausente: \(url.path)") }
        return CTFontCreateWithFontDescriptor(first, size, nil)
    }
    static func condensed(_ size: CGFloat) -> CTFont { load("BarlowCondensed-MediumItalic.ttf", size: size) }
    static func text(_ size: CGFloat) -> CTFont { load("Barlow-Regular.ttf", size: size) }
    static func medium(_ size: CGFloat) -> CTFont { load("Barlow-Medium.ttf", size: size) }
}

/// Uma linha de texto com a linha de base no ponto (o contexto é y-para-baixo).
@discardableResult
func drawLine(_ string: String, font: CTFont, color: CGColor, baseline: CGPoint,
              centered: Bool = false, kern: CGFloat = 0, in context: CGContext) -> CGFloat {
    let line = CTLineCreateWithAttributedString(NSAttributedString(string: string, attributes: [
        NSAttributedString.Key(kCTFontAttributeName as String): font,
        NSAttributedString.Key(kCTForegroundColorAttributeName as String): color,
        NSAttributedString.Key(kCTKernAttributeName as String): kern,
    ]))
    let width = CGFloat(CTLineGetTypographicBounds(line, nil, nil, nil))
    context.saveGState()
    context.translateBy(x: centered ? baseline.x - width / 2 : baseline.x, y: baseline.y)
    context.scaleBy(x: 1, y: -1)
    context.textPosition = .zero
    CTLineDraw(line, context)
    context.restoreGState()
    return width
}

// MARK: - Assinaturas

/// "FALCÃO TOKEN ROUTER": o nome do app vem depois de FALCÃO, na mesma linha, na
/// mesma fonte e mais leve (manual, seção 3). Unidades do SVG horizontal: o
/// símbolo ocupa x 6–200 e y 6–195; o nome, x 242–589 com a base em y 160; o nome
/// do app começa em x 623, no corpo 137.
func drawHorizontalSignature(origin: CGPoint, symbolHeight: CGFloat, color: CGColor,
                             in context: CGContext) {
    let k = symbolHeight / 189
    func at(_ u: CGFloat, _ v: CGFloat) -> CGPoint { CGPoint(x: origin.x + (u - 6) * k, y: origin.y + (v - 6) * k) }
    draw(BrandMark.symbol, box: BrandMark.symbolBox,
         into: CGRect(origin: at(6, 6), size: CGSize(width: 194 * k, height: 189 * k)), color: color, in: context)
    draw(BrandMark.wordmark, box: BrandMark.nameBox,
         into: CGRect(origin: at(242, 42.04), size: CGSize(width: 347 * k, height: 118 * k)), color: color, in: context)
    drawLine("TOKEN ROUTER", font: BrandFont.condensed(137 * k), color: color,
             baseline: at(623, 160), kern: 2 * k, in: context)
}

/// A mesma assinatura empilhada: o símbolo maior, centrado sobre a linha do nome.
/// Unidades: a linha mede 1094,67 (nome 347 + espaço 34 + nome do app); o
/// símbolo vai a 1,78×, e a base do texto fica 490 abaixo do topo.
func drawStackedSignature(centerX: CGFloat, top: CGFloat, lineWidth: CGFloat, color: CGColor,
                          in context: CGContext) {
    let k = lineWidth / 1094.67
    let symbolSize = CGSize(width: 194 * 1.78 * k, height: 189 * 1.78 * k)
    draw(BrandMark.symbol, box: BrandMark.symbolBox,
         into: CGRect(origin: CGPoint(x: centerX - symbolSize.width / 2, y: top), size: symbolSize),
         color: color, in: context)
    let left = centerX - lineWidth / 2
    draw(BrandMark.wordmark, box: BrandMark.nameBox,
         into: CGRect(x: left, y: top + 372 * k, width: 347 * k, height: 118 * k), color: color, in: context)
    drawLine("TOKEN ROUTER", font: BrandFont.condensed(137 * k), color: color,
             baseline: CGPoint(x: left + 381 * k, y: top + 490 * k), kern: 2 * k, in: context)
}

// MARK: - Contexto

/// Contexto y-para-baixo, para casar com a convenção dos SVGs da marca e com a
/// do Finder, que posiciona os ícones do DMG a partir do topo da janela.
func makeContext(width: Int, height: Int) -> CGContext {
    let context = CGContext(data: nil, width: width, height: height,
                            bitsPerComponent: 8, bytesPerRow: 0,
                            space: CGColorSpace(name: CGColorSpace.sRGB)!,
                            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    context.translateBy(x: 0, y: CGFloat(height))
    context.scaleBy(x: 1, y: -1)
    context.setAllowsAntialiasing(true)
    return context
}

func writePNG(_ context: CGContext, to url: URL) {
    let image = context.makeImage()!
    let destination = CGImageDestinationCreateWithURL(
        url as CFURL, UTType.png.identifier as CFString, 1, nil)!
    CGImageDestinationAddImage(destination, image, nil)
    guard CGImageDestinationFinalize(destination) else {
        fatalError("falhou ao escrever \(url.path)")
    }
}

/// Desenha texto num contexto já invertido. O CoreText desenha de baixo para
/// cima, então a inversão precisa ser desfeita só em volta da linha.
func drawText(_ string: String, font: NSFont, color: CGColor,
              centeredAt point: CGPoint, in context: CGContext) {
    let line = CTLineCreateWithAttributedString(NSAttributedString(
        string: string,
        attributes: [.font: font, .foregroundColor: NSColor(cgColor: color)!]))
    let bounds = CTLineGetBoundsWithOptions(line, .useOpticalBounds)

    context.saveGState()
    context.translateBy(x: point.x - bounds.width / 2, y: point.y + bounds.height / 2)
    context.scaleBy(x: 1, y: -1)
    context.textPosition = .zero
    CTLineDraw(line, context)
    context.restoreGState()
}

/// Mesma coisa, ancorado à esquerda em vez de centrado.
func drawText(_ string: String, font: NSFont, color: CGColor,
              leftAt point: CGPoint, in context: CGContext) {
    let line = CTLineCreateWithAttributedString(NSAttributedString(
        string: string,
        attributes: [.font: font, .foregroundColor: NSColor(cgColor: color)!]))
    let bounds = CTLineGetBoundsWithOptions(line, .useOpticalBounds)

    context.saveGState()
    context.translateBy(x: point.x, y: point.y + bounds.height / 2)
    context.scaleBy(x: 1, y: -1)
    context.textPosition = .zero
    CTLineDraw(line, context)
    context.restoreGState()
}

// MARK: - Formas

/// Superelipse — a família de curva do ícone do macOS, cujo canto é contínuo em
/// vez de um arco de círculo colado numa reta. Um `roundedRect` comum entrega a
/// silhueta errada e é o detalhe que faz um ícone parecer de fora.
func squircle(in rect: CGRect, exponent: Double = 5) -> CGPath {
    let path = CGMutablePath()
    let a = rect.width / 2, b = rect.height / 2
    let center = CGPoint(x: rect.midX, y: rect.midY)
    let samples = 720

    for step in 0...samples {
        let t = Double(step) / Double(samples) * 2 * .pi
        let cosT = cos(t), sinT = sin(t)
        let x = center.x + a * CGFloat(copysign(pow(abs(cosT), 2 / exponent), cosT))
        let y = center.y + b * CGFloat(copysign(pow(abs(sinT), 2 / exponent), sinT))
        step == 0 ? path.move(to: CGPoint(x: x, y: y)) : path.addLine(to: CGPoint(x: x, y: y))
    }
    path.closeSubpath()
    return path
}

// MARK: - O ícone

func drawIcon(side: CGFloat, in context: CGContext) {
    // Abaixo de 64px a folga e a sombra caem, e o símbolo cresce: a 16px (Itens de
    // Login, diálogos do Gatekeeper) a proporção grande vira um borrão.
    let compact = side < 64

    // A placa ocupa 82,4% da tela, a proporção histórica do ícone do macOS. O
    // resto é a folga que o sistema espera — e que impede que a máscara do
    // macOS 26 corte o canto do desenho.
    let inset = side * (compact ? 0.045 : 0.088)
    let plate = CGRect(x: inset, y: inset, width: side - inset * 2, height: side - inset * 2)
    let shape = squircle(in: plate)

    if !compact {
        context.saveGState()
        context.setShadow(offset: CGSize(width: 0, height: -side * 0.014),
                          blur: side * 0.028, color: Palette.shadow)
        context.addPath(shape)
        context.setFillColor(Palette.navy)
        context.fillPath()
        context.restoreGState()
    }
    context.addPath(shape)
    context.setFillColor(Palette.navy)
    context.fillPath()

    // Marca-mãe no ícone: símbolo branco sobre marinho, a 66% da largura — a
    // proporção do ícone arredondado da ferramenta da marca.
    let width = plate.width * (compact ? 0.74 : 0.66)
    let height = width * BrandMark.symbolBox.height / BrandMark.symbolBox.width
    draw(BrandMark.symbol, box: BrandMark.symbolBox,
         into: CGRect(x: plate.midX - width / 2, y: plate.midY - height / 2, width: width, height: height),
         color: Palette.white, in: context)
}

// MARK: - O fundo do DMG

func drawInstallerBackground(size: CGSize, scale: CGFloat, in context: CGContext) {
    let rect = CGRect(origin: .zero, size: size)
    context.saveGState()
    context.scaleBy(x: scale, y: scale)

    context.setFillColor(Palette.mist)
    context.fill(rect)

    // Só a assinatura. A instrução de arrastar foi removida de propósito: o Finder
    // já mostra o ícone, a seta e a pasta de destino **com o nome no idioma de
    // quem abriu**, e a frase obrigaria a gerar uma arte por idioma.
    let symbolHeight: CGFloat = 40
    let signatureWidth = (1336.67 - 6) * symbolHeight / 189
    drawHorizontalSignature(origin: CGPoint(x: rect.midX - signatureWidth / 2, y: 40),
                            symbolHeight: symbolHeight, color: Palette.navy, in: context)

    // Um halo suave atrás de onde o Finder vai pôr o app: sem ele, o olho não
    // tem por onde começar numa janela quase toda clara.
    context.saveGState()
    let halo = CGGradient(colorsSpace: CGColorSpace(name: CGColorSpace.sRGB)!,
                          colors: [Palette.hex(0x0B2D4C, alpha: 0.09), Palette.hex(0x0B2D4C, alpha: 0)] as CFArray,
                          locations: [0, 1])!
    let haloCenter = CGPoint(x: installerAppCenterX, y: installerIconCenterY)
    context.drawRadialGradient(halo, startCenter: haloCenter, startRadius: 0,
                               endCenter: haloCenter, endRadius: 132, options: [])
    context.restoreGState()

    // A seta liga as duas posições que o osascript fixa no Finder. Se aquelas
    // mudarem, esta precisa mudar junto — por isso as duas leem as constantes
    // abaixo. No sentido do voo: da esquerda para a direita.
    let y = installerIconCenterY
    let from = installerAppCenterX + 78
    let to = installerFolderCenterX - 78
    let arrow = Palette.feather

    context.setStrokeColor(arrow)
    context.setLineWidth(2.5)
    context.setLineCap(.round)
    context.move(to: CGPoint(x: from, y: y))
    context.addLine(to: CGPoint(x: to - 9, y: y))
    context.strokePath()

    context.setFillColor(arrow)
    context.move(to: CGPoint(x: to, y: y))
    context.addLine(to: CGPoint(x: to - 14, y: y - 8))
    context.addLine(to: CGPoint(x: to - 14, y: y + 8))
    context.closePath()
    context.fillPath()

    // Rodapé com os dois requisitos que fazem o app simplesmente não abrir. É a
    // informação mais útil que cabe aqui, e o instalador é o último momento em
    // que alguém a lê antes de concluir que o app está quebrado. Em inglês,
    // porque o DMG é servido para todo mundo a partir de um arquivo só.
    drawLine("Requires macOS 26 or later  ·  Intel and Apple Silicon", font: BrandFont.text(12),
             color: Palette.slate, baseline: CGPoint(x: rect.midX, y: 356), centered: true, in: context)

    context.restoreGState()
}

// MARK: - Banner e card social

/// Em inglês: banner e card social são a página do projeto, servida a partir de
/// um arquivo só para quem chegar de qualquer lugar. A frase é a do About do
/// repositório.
let tagline = "Claude Code accounts in groups that take turns by themselves."

/// O fundo da marca: marinho chapado, com o símbolo ampliado ao fundo em marinho
/// mais claro (splash, modelo 2 do manual), sangrando pela borda.
func drawBrandSurface(size: CGSize, watermark: CGRect, in context: CGContext) {
    let rect = CGRect(origin: .zero, size: size)
    context.setFillColor(Palette.navy)
    context.fill(rect)
    context.saveGState()
    context.clip(to: rect)
    draw(BrandMark.symbol, box: BrandMark.symbolBox, into: watermark, color: Palette.navyLight, in: context)
    context.restoreGState()
}

/// Banner do README: 1280x360, desenhado em 2x.
func drawBanner(in context: CGContext, scale: CGFloat) {
    let size = CGSize(width: 1280, height: 360)
    context.saveGState()
    context.scaleBy(x: scale, y: scale)

    drawBrandSurface(size: size, watermark: CGRect(x: 905, y: -95, width: 540, height: 526), in: context)
    drawHorizontalSignature(origin: CGPoint(x: 96, y: 78), symbolHeight: 112, color: Palette.white, in: context)
    drawLine(tagline, font: BrandFont.text(25), color: Palette.cloud,
             baseline: CGPoint(x: 100, y: 250), in: context)
    drawLine("macOS  ·  Windows  ·  MIT", font: BrandFont.medium(16), color: Palette.feather,
             baseline: CGPoint(x: 100, y: 290), in: context)

    context.restoreGState()
}

/// Card social do GitHub: 1280x640, o tamanho que a Open Graph espera. É o que
/// aparece quando o link é colado no Slack, no WhatsApp ou no X — ou seja, o
/// jeito mais provável de alguém encontrar o projeto pela primeira vez.
func drawSocialPreview(in context: CGContext) {
    let size = CGSize(width: 1280, height: 640)
    drawBrandSurface(size: size, watermark: CGRect(x: 905, y: -150, width: 600, height: 585), in: context)
    drawStackedSignature(centerX: size.width / 2, top: 104, lineWidth: 660, color: Palette.white, in: context)
    drawLine(tagline, font: BrandFont.text(27), color: Palette.cloud,
             baseline: CGPoint(x: size.width / 2, y: 478), centered: true, in: context)
    drawLine("github.com/Falkzera/falcao-token-router", font: BrandFont.medium(20), color: Palette.feather,
             baseline: CGPoint(x: size.width / 2, y: 548), centered: true, in: context)
}

// MARK: - Moldura do screenshot

/// Emoldura uma captura crua do painel para o README.
///
/// Uma captura de janela do macOS vem com a faixa de sombra em volta, e naquela
/// faixa aparece o que estava atrás — outras janelas, o wallpaper. Recortar no
/// painel resolve, mas expõe os cantos arredondados, que são translúcidos e
/// carregam a cor do que estava atrás. Por isso a moldura: o recorte é
/// mascarado no mesmo raio do painel e assentado sobre a superfície da marca,
/// que é o que faz os cantos lerem como intenção em vez de sujeira.
func frameScreenshot(rawPath: String) -> CGContext {
    guard let image = NSImage(contentsOfFile: rawPath),
          let source = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
        fatalError("não consegui ler a captura em \(rawPath)")
    }

    let bounds = detectPanel(in: source)
    guard let panel = source.cropping(to: bounds) else {
        fatalError("recorte inválido: \(bounds)")
    }

    let padding: CGFloat = 64
    let radius: CGFloat = 36
    let size = CGSize(width: CGFloat(panel.width) + padding * 2,
                      height: CGFloat(panel.height) + padding * 2)
    let context = makeContext(width: Int(size.width), height: Int(size.height))

    drawBrandSurface(size: size,
                     watermark: CGRect(x: size.width * 0.62, y: -size.height * 0.1,
                                       width: size.width * 0.6, height: size.width * 0.6 * 189 / 194),
                     in: context)

    let frame = CGRect(x: padding, y: padding,
                       width: CGFloat(panel.width), height: CGFloat(panel.height))
    let rounded = CGPath(roundedRect: frame, cornerWidth: radius, cornerHeight: radius,
                         transform: nil)

    // A sombra é pintada como forma própria antes do recorte. Desenhar a imagem
    // já clipada com sombra ligada não funciona: o clip corta a sombra junto.
    context.saveGState()
    context.setShadow(offset: CGSize(width: 0, height: 18), blur: 44,
                      color: CGColor(gray: 0, alpha: 0.45))
    context.addPath(rounded)
    context.setFillColor(CGColor(gray: 0, alpha: 1))
    context.fillPath()
    context.restoreGState()

    context.saveGState()
    context.addPath(rounded)
    context.clip()
    // O contexto é y-para-baixo; `draw` assume y-para-cima, então a imagem sai
    // de cabeça para baixo sem desfazer a inversão em volta dela.
    context.translateBy(x: 0, y: frame.maxY)
    context.scaleBy(x: 1, y: -1)
    context.draw(panel, in: CGRect(x: frame.minX, y: 0,
                                   width: frame.width, height: frame.height))
    context.restoreGState()

    return context
}

/// Acha o painel dentro da captura pela diferença de luminância entre ele e a
/// faixa de sombra. Detectar em vez de fixar números deixa a moldura servir a
/// próxima captura, que vai ter outro tamanho.
func detectPanel(in image: CGImage) -> CGRect {
    let w = image.width, h = image.height
    var pixels = [UInt8](repeating: 0, count: w * h * 4)
    let scan = CGContext(data: &pixels, width: w, height: h, bitsPerComponent: 8,
                         bytesPerRow: w * 4, space: CGColorSpace(name: CGColorSpace.sRGB)!,
                         bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    scan.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))

    func luma(_ x: Int, _ y: Int) -> Int {
        let i = (y * w + x) * 4
        return (Int(pixels[i]) * 30 + Int(pixels[i + 1]) * 59 + Int(pixels[i + 2]) * 11) / 100
    }
    let threshold = 22
    let midY = h / 2, midX = w / 2

    var left = 0, right = w - 1, bottom = 0, top = h - 1
    while left < w && luma(left, midY) < threshold { left += 1 }
    while right > left && luma(right, midY) < threshold { right -= 1 }
    while bottom < h && luma(midX, bottom) < threshold { bottom += 1 }
    while top > bottom && luma(midX, top) < threshold { top -= 1 }

    return CGRect(x: left, y: bottom, width: right - left + 1, height: top - bottom + 1)
}

/// Compartilhadas com o Scripts/dmg.sh: a arte desenha a seta entre estes dois
/// pontos, e o Finder coloca os ícones neles.
let installerAppCenterX: CGFloat = 172
let installerFolderCenterX: CGFloat = 468
let installerIconCenterY: CGFloat = 214
let installerWindow = CGSize(width: 640, height: 396)

// MARK: - Saída

// `@main`, compilado com `-parse-as-library` (icon.sh): as constantes globais
// do arquivo não viram código de script.
@main
enum IconGen {
    static func main() throws {
        let outputRoot = URL(fileURLWithPath: CommandLine.arguments.count > 1
                             ? CommandLine.arguments[1] : "dist")
        let iconset = outputRoot.appendingPathComponent("AppIcon.iconset")
        try FileManager.default.createDirectory(at: iconset, withIntermediateDirectories: true)

        // Cada tamanho é renderizado no seu próprio tamanho, não reduzido do
        // maior: a 16px o traço fino some se vier de uma redução.
        for (pixels, name) in [(16, "icon_16x16"), (32, "icon_16x16@2x"),
                               (32, "icon_32x32"), (64, "icon_32x32@2x"),
                               (128, "icon_128x128"), (256, "icon_128x128@2x"),
                               (256, "icon_256x256"), (512, "icon_256x256@2x"),
                               (512, "icon_512x512"), (1024, "icon_512x512@2x")] {
            let context = makeContext(width: pixels, height: pixels)
            drawIcon(side: CGFloat(pixels), in: context)
            writePNG(context, to: iconset.appendingPathComponent("\(name).png"))
        }

        for (scale, name) in [(CGFloat(1), "installer-background.png"),
                              (CGFloat(2), "installer-background@2x.png")] {
            let context = makeContext(width: Int(installerWindow.width * scale),
                                      height: Int(installerWindow.height * scale))
            drawInstallerBackground(size: installerWindow, scale: scale, in: context)
            writePNG(context, to: outputRoot.appendingPathComponent(name))
        }

        // Banner em 2x: o GitHub reduz a imagem para a largura do conteúdo, e
        // reduzir é o que preserva nitidez em tela retina. O card social vai em
        // 1x, no tamanho exato que a Open Graph espera.
        let banner = makeContext(width: 2560, height: 720)
        drawBanner(in: banner, scale: 2)
        writePNG(banner, to: outputRoot.appendingPathComponent("banner.png"))

        let social = makeContext(width: 1280, height: 640)
        drawSocialPreview(in: social)
        writePNG(social, to: outputRoot.appendingPathComponent("social-preview.png"))

        // A captura crua é opcional: só quem tirou uma nova passa o caminho.
        if CommandLine.arguments.count > 2 {
            let framed = frameScreenshot(rawPath: CommandLine.arguments[2])
            writePNG(framed, to: outputRoot.appendingPathComponent("panel.png"))
        }

        print("==> arte gerada em \(outputRoot.path)")
    }
}
