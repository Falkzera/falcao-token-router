//
// Gera a arte do app: o .iconset e o fundo do DMG.
//
// Compilado junto com Sources/CCUsageCore/Presentation/GaugeGeometry.swift —
// veja Scripts/icon.sh. O anel daqui é literalmente o mesmo código do anel da
// barra de menu; sem isso, "identidade visual" viraria dois desenhos parecidos
// que alguém tem que lembrar de manter em sincronia.
//
// Uso: icongen <diretório de saída>

import AppKit
import CoreGraphics
import CoreText
import Foundation
import ImageIO
import UniformTypeIdentifiers

// MARK: - Paleta

enum Palette {
    /// O mesmo verde dessaturado do UsageColor.calm — a cor do estado normal.
    /// O ícone é o app parado, não o app alarmado.
    static let markTop = CGColor(red: 0.42, green: 0.70, blue: 0.52, alpha: 1)
    static let markBottom = CGColor(red: 0.20, green: 0.40, blue: 0.30, alpha: 1)
    static let ring = CGColor(gray: 1, alpha: 0.96)
    static let track = CGColor(gray: 1, alpha: 0.26)
    static let gloss = CGColor(gray: 1, alpha: 0.34)
    static let shadow = CGColor(gray: 0, alpha: 0.28)

    /// Superfície escura da marca, usada no banner e no card social. Mais
    /// funda que a do ícone de propósito: ali o verde é o objeto, aqui é o
    /// fundo atrás de texto branco.
    static let deepTop = CGColor(red: 0.24, green: 0.45, blue: 0.34, alpha: 1)
    static let deepBottom = CGColor(red: 0.12, green: 0.26, blue: 0.20, alpha: 1)

    static let paper = CGColor(red: 0.976, green: 0.980, blue: 0.976, alpha: 1)
    static let paperEdge = CGColor(red: 0.918, green: 0.937, blue: 0.925, alpha: 1)
    static let ink = CGColor(gray: 0.13, alpha: 1)
    static let inkFaint = CGColor(gray: 0.45, alpha: 1)
}

/// Quanto do anel o ícone mostra.
///
/// 62%, e não um número redondo, porque `UsageColor.calmThreshold` é 0.66: este
/// é o arco mais cheio que ainda cabe no verde. Um ícone permanentemente âmbar
/// seria um alarme falso na pasta de aplicativos.
let iconFraction = 0.62

// MARK: - Contexto

/// Contexto y-para-baixo, para casar com a convenção do `GaugeGeometry` e com a
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

func fill(_ path: CGPath, from top: CGColor, to bottom: CGColor,
          in rect: CGRect, context: CGContext) {
    context.saveGState()
    context.addPath(path)
    context.clip()
    let gradient = CGGradient(colorsSpace: CGColorSpace(name: CGColorSpace.sRGB)!,
                              colors: [top, bottom] as CFArray, locations: [0, 1])!
    context.drawLinearGradient(gradient,
                               start: CGPoint(x: rect.midX, y: rect.minY),
                               end: CGPoint(x: rect.midX, y: rect.maxY),
                               options: [])
    context.restoreGState()
}

// MARK: - O ícone

func drawIcon(side: CGFloat, in context: CGContext) {
    // Abaixo de 64px o desenho engrossa em vez de encolher junto. A 16px, as
    // proporções do tamanho grande entregam um anel de 1,4px que o
    // antialiasing dissolve num borrão — e 16px é o tamanho dos Itens de Login
    // e dos diálogos do Gatekeeper, onde muita gente vê o app pela primeira
    // vez. Sombra e brilho também saem: a essa escala viram sujeira, não
    // volume.
    let compact = side < 64

    // A arte ocupa 82,4% da tela, a proporção histórica do ícone do macOS. O
    // resto é a folga que o sistema espera — e que impede que a máscara do
    // macOS 26 corte o canto do desenho. No compacto a folga cai, porque a
    // sombra que ela acomodava não é mais desenhada.
    let inset = side * (compact ? 0.045 : 0.088)
    let plate = CGRect(x: inset, y: inset, width: side - inset * 2, height: side - inset * 2)
    let shape = squircle(in: plate)

    if !compact {
        context.saveGState()
        context.setShadow(offset: CGSize(width: 0, height: -side * 0.014),
                          blur: side * 0.028, color: Palette.shadow)
        context.addPath(shape)
        context.setFillColor(Palette.markBottom)
        context.fillPath()
        context.restoreGState()
    }

    fill(shape, from: Palette.markTop, to: Palette.markBottom, in: plate, context: context)

    // Brilho superior: a metade de cima da placa, esmaecendo para baixo. É o que
    // dá a leitura de vidro sem depender do material dinâmico do sistema, que
    // só o Icon Composer produz.
    if !compact {
    context.saveGState()
    context.addPath(shape)
    context.clip()
    let glossRect = CGRect(x: plate.minX, y: plate.minY,
                           width: plate.width, height: plate.height * 0.58)
    let gloss = CGGradient(colorsSpace: CGColorSpace(name: CGColorSpace.sRGB)!,
                           colors: [Palette.gloss, CGColor(gray: 1, alpha: 0)] as CFArray,
                           locations: [0, 1])!
    context.drawLinearGradient(gloss,
                               start: CGPoint(x: glossRect.midX, y: glossRect.minY),
                               end: CGPoint(x: glossRect.midX, y: glossRect.maxY),
                               options: [])
    context.restoreGState()

    // Fio de luz na borda de cima, o realce que separa a placa do fundo.
    context.saveGState()
    context.addPath(shape)
    context.clip()
    context.addPath(shape)
    context.setStrokeColor(CGColor(gray: 1, alpha: 0.45))
    context.setLineWidth(side * 0.006)
    context.strokePath()
    context.restoreGState()
    }

    // O anel, pelo mesmo GaugeGeometry que a barra de menu usa.
    let ringSide = plate.width * (compact ? 0.68 : 0.54)
    // No compacto o traço nunca fica abaixo de 2px: menos que isso e o anel
    // deixa de ler como anel.
    let lineWidth = max(compact ? 2 : 0, ringSide * (compact ? 0.24 : 0.155))
    drawRing(in: CGRect(x: plate.midX - ringSide / 2, y: plate.midY - ringSide / 2,
                        width: ringSide, height: ringSide),
             lineWidth: lineWidth, fraction: iconFraction,
             ring: Palette.ring, track: Palette.track, in: context)
}

/// O anel isolado. Ícone, banner e card social passam por aqui — é o que faz
/// deles a mesma marca em vez de três desenhos parecidos.
func drawRing(in rect: CGRect, lineWidth: CGFloat, fraction: Double,
              ring: CGColor, track: CGColor?, in context: CGContext) {
    context.saveGState()
    context.setLineCap(.round)
    context.setLineWidth(lineWidth)

    if let track {
        context.addPath(GaugeGeometry.trackPath(in: rect, lineWidth: lineWidth))
        context.setStrokeColor(track)
        context.strokePath()
    }

    context.addPath(GaugeGeometry.arcPath(in: rect, lineWidth: lineWidth, fraction: fraction))
    context.setStrokeColor(ring)
    context.strokePath()
    context.restoreGState()
}

// MARK: - O fundo do DMG

func drawInstallerBackground(size: CGSize, scale: CGFloat, in context: CGContext) {
    let rect = CGRect(origin: .zero, size: size)
    context.saveGState()
    context.scaleBy(x: scale, y: scale)

    let gradient = CGGradient(colorsSpace: CGColorSpace(name: CGColorSpace.sRGB)!,
                              colors: [Palette.paper, Palette.paperEdge] as CFArray,
                              locations: [0, 1])!
    context.drawLinearGradient(gradient,
                               start: CGPoint(x: rect.midX, y: 0),
                               end: CGPoint(x: rect.midX, y: rect.height),
                               options: [])

    // Só o nome. A instrução de arrastar foi removida de propósito: o Finder já
    // mostra o ícone, a seta e a pasta de destino **com o nome no idioma de
    // quem abriu** — "Applications", "Aplicativos", "アプリケーション". A frase
    // repetia em português o que a janela já diz na língua certa, e obrigaria a
    // gerar uma arte por idioma para um DMG que é montado uma vez só.
    drawText("Falcão Token Router", font: .systemFont(ofSize: 22, weight: .semibold),
             color: Palette.ink, centeredAt: CGPoint(x: rect.midX, y: 62), in: context)

    // Um halo suave atrás de onde o Finder vai pôr o app: sem ele, o olho não
    // tem por onde começar numa janela que é quase toda branca.
    context.saveGState()
    let halo = CGGradient(colorsSpace: CGColorSpace(name: CGColorSpace.sRGB)!,
                          colors: [CGColor(red: 0.42, green: 0.70, blue: 0.52, alpha: 0.13),
                                   CGColor(red: 0.42, green: 0.70, blue: 0.52, alpha: 0)] as CFArray,
                          locations: [0, 1])!
    let haloCenter = CGPoint(x: installerAppCenterX, y: installerIconCenterY)
    context.drawRadialGradient(halo, startCenter: haloCenter, startRadius: 0,
                               endCenter: haloCenter, endRadius: 132, options: [])
    context.restoreGState()

    // A seta liga as duas posições que o osascript fixa no Finder. Se aquelas
    // mudarem, esta precisa mudar junto — por isso as duas leem as constantes
    // abaixo.
    let y = installerIconCenterY
    let from = installerAppCenterX + 78
    let to = installerFolderCenterX - 78
    let arrow = CGColor(red: 0.42, green: 0.62, blue: 0.50, alpha: 0.85)

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
    // que alguém a lê antes de concluir que o app está quebrado.
    // O rodapé carrega informação real, e por isso fica — em inglês, porque o
    // DMG é servido para todo mundo a partir de um arquivo só.
    drawText("Requires macOS 26 or later  ·  Intel and Apple Silicon",
             font: .systemFont(ofSize: 11, weight: .regular),
             color: CGColor(gray: 0.58, alpha: 1),
             centeredAt: CGPoint(x: rect.midX, y: 352), in: context)

    context.restoreGState()
}

// MARK: - Banner e card social

/// Em inglês: banner e card social são a página do projeto, servida a partir de
/// um arquivo só para quem chegar de qualquer lugar.
let tagline = "How much of your plan you've used — and where that number came from."

/// Fundo escuro da marca, com um anel enorme sangrando pela borda. O anel
/// gigante é o mesmo `drawRing` — a marca aparece duas vezes, em escalas
/// diferentes, sem virar dois desenhos.
func drawBrandSurface(size: CGSize, watermark: CGPoint, watermarkSide: CGFloat,
                      in context: CGContext) {
    let rect = CGRect(origin: .zero, size: size)
    let gradient = CGGradient(colorsSpace: CGColorSpace(name: CGColorSpace.sRGB)!,
                              colors: [Palette.deepTop, Palette.deepBottom] as CFArray,
                              locations: [0, 1])!
    context.drawLinearGradient(gradient,
                               start: CGPoint(x: rect.midX, y: 0),
                               end: CGPoint(x: rect.midX, y: rect.height),
                               options: [])

    context.saveGState()
    context.clip(to: rect)
    drawRing(in: CGRect(x: watermark.x - watermarkSide / 2,
                        y: watermark.y - watermarkSide / 2,
                        width: watermarkSide, height: watermarkSide),
             lineWidth: watermarkSide * 0.115, fraction: 1,
             ring: CGColor(gray: 1, alpha: 0.075), track: nil, in: context)
    context.restoreGState()
}

/// Banner do README: 1280x360, desenhado em 2x.
func drawBanner(in context: CGContext, scale: CGFloat) {
    let size = CGSize(width: 1280, height: 360)
    context.saveGState()
    context.scaleBy(x: scale, y: scale)

    drawBrandSurface(size: size, watermark: CGPoint(x: 1285, y: 178),
                     watermarkSide: 560, in: context)

    drawRing(in: CGRect(x: 96, y: 116, width: 128, height: 128),
             lineWidth: 128 * 0.155, fraction: iconFraction,
             ring: CGColor(gray: 1, alpha: 0.97),
             track: CGColor(gray: 1, alpha: 0.22), in: context)

    drawText("Falcão Token Router", font: .systemFont(ofSize: 50, weight: .semibold),
             color: CGColor(gray: 1, alpha: 1), leftAt: CGPoint(x: 262, y: 140), in: context)
    drawText(tagline, font: .systemFont(ofSize: 21, weight: .regular),
             color: CGColor(gray: 1, alpha: 0.82), leftAt: CGPoint(x: 264, y: 202), in: context)
    drawText("macOS 26+  ·  menu bar  ·  MIT",
             font: .systemFont(ofSize: 15, weight: .medium),
             color: CGColor(gray: 1, alpha: 0.5), leftAt: CGPoint(x: 264, y: 246), in: context)

    context.restoreGState()
}

/// Card social do GitHub: 1280x640, o tamanho que a Open Graph espera. É o que
/// aparece quando o link é colado no Slack, no WhatsApp ou no X — ou seja, o
/// jeito mais provável de alguém encontrar o projeto pela primeira vez.
func drawSocialPreview(in context: CGContext) {
    let size = CGSize(width: 1280, height: 640)

    drawBrandSurface(size: size, watermark: CGPoint(x: 1215, y: 90),
                     watermarkSide: 620, in: context)

    drawRing(in: CGRect(x: 640 - 88, y: 168, width: 176, height: 176),
             lineWidth: 176 * 0.155, fraction: iconFraction,
             ring: CGColor(gray: 1, alpha: 0.97),
             track: CGColor(gray: 1, alpha: 0.22), in: context)

    drawText("Falcão Token Router", font: .systemFont(ofSize: 60, weight: .semibold),
             color: CGColor(gray: 1, alpha: 1),
             centeredAt: CGPoint(x: size.width / 2, y: 404), in: context)
    drawText(tagline, font: .systemFont(ofSize: 25, weight: .regular),
             color: CGColor(gray: 1, alpha: 0.84),
             centeredAt: CGPoint(x: size.width / 2, y: 478), in: context)
    drawText("github.com/Falkzera/falcao-token-router",
             font: .systemFont(ofSize: 19, weight: .medium),
             color: CGColor(gray: 1, alpha: 0.46),
             centeredAt: CGPoint(x: size.width / 2, y: 552), in: context)
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

    drawBrandSurface(size: size, watermark: CGPoint(x: size.width * 0.86, y: size.height * 0.12),
                     watermarkSide: size.width * 0.72, in: context)

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

// `@main` em vez de código solto no topo: com dois arquivos de entrada, o
// swiftc só aceita instruções de topo num arquivo chamado main.swift.
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
