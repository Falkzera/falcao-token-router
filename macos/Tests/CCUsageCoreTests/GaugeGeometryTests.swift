import CoreGraphics
import Testing
@testable import CCUsageCore

/// O menu bar rasteriza um bitmap por passo, então a quantização define quantos
/// desenhos distintos existem — e quais frações são indistinguíveis entre si.
private let steps = 20

// MARK: - Varredura do arco

@Test func sweepIsZeroWithoutUsage() {
    #expect(GaugeGeometry.sweepDegrees(fraction: 0) == 0)
}

@Test func sweepIsFullTurnAtTheCeiling() {
    #expect(GaugeGeometry.sweepDegrees(fraction: 1) == 360)
}

@Test func sweepIsProportionalInBetween() {
    #expect(GaugeGeometry.sweepDegrees(fraction: 0.25) == 90)
}

/// O consumo passa do teto calibrado com alguma frequência — o teto é uma
/// estimativa do próprio histórico, não um limite imposto. O anel para de girar
/// em vez de dar a segunda volta.
@Test func sweepClampsAboveTheCeiling() {
    #expect(GaugeGeometry.sweepDegrees(fraction: 1.4) == 360)
}

@Test func sweepClampsBelowZero() {
    #expect(GaugeGeometry.sweepDegrees(fraction: -0.5) == 0)
}

// MARK: - Quantização

@Test func quantizeCollapsesNeighborsIntoOneStep() {
    #expect(GaugeGeometry.quantize(0.31, steps: steps)
            == GaugeGeometry.quantize(0.34, steps: steps))
}

/// Anel fechado quer dizer "bateu o teto". Arredondar 99,9% para cima entrega
/// esse desenho antes da hora — é a mesma classe de mentira que o app existe
/// para não contar.
@Test func quantizeNeverReportsFullBeforeTheCeiling() {
    #expect(GaugeGeometry.quantize(0.999, steps: steps) == steps - 1)
}

@Test func quantizeReportsFullOnlyAtTheCeiling() {
    #expect(GaugeGeometry.quantize(1.0, steps: steps) == steps)
}

/// Simétrico ao caso acima: anel vazio quer dizer "não gastei nada". Qualquer
/// consumo, por menor que seja, precisa desenhar alguma coisa.
@Test func quantizeDrawsSomethingForAnyUsage() {
    #expect(GaugeGeometry.quantize(0.0001, steps: steps) == 1)
}

@Test func quantizeIsEmptyOnlyWithoutUsage() {
    #expect(GaugeGeometry.quantize(0, steps: steps) == 0)
}

// MARK: - Path do arco

/// Convenção y-para-baixo, a mesma do SwiftUI: um quarto de volta a partir das
/// 12h ocupa exatamente o quadrante superior direito. É o teste que trava o
/// ponto de partida e o sentido de giro — os dois erros de sinal que passariam
/// despercebidos até alguém olhar o ícone pronto.
@Test func quarterArcCoversTopRightQuadrant() {
    let box = GaugeGeometry.arcPath(in: CGRect(x: 0, y: 0, width: 100, height: 100),
                                    lineWidth: 0, fraction: 0.25).boundingBox
    #expect(abs(box.minX - 50) < 0.5)
    #expect(abs(box.minY - 0) < 0.5)
    #expect(abs(box.maxX - 100) < 0.5)
    #expect(abs(box.maxY - 50) < 0.5)
}

@Test func fullArcClosesTheWholeRing() {
    let box = GaugeGeometry.arcPath(in: CGRect(x: 0, y: 0, width: 100, height: 100),
                                    lineWidth: 0, fraction: 1).boundingBox
    #expect(abs(box.width - 100) < 0.5)
    #expect(abs(box.height - 100) < 0.5)
}

@Test func emptyArcDrawsNothing() {
    #expect(GaugeGeometry.arcPath(in: CGRect(x: 0, y: 0, width: 100, height: 100),
                                  lineWidth: 4, fraction: 0).isEmpty)
}

/// O traço cresce para os dois lados da linha de centro, então o raio precisa
/// recuar metade da espessura — senão o anel vaza pela borda do ícone.
@Test func strokeStaysInsideTheRect() {
    let box = GaugeGeometry.arcPath(in: CGRect(x: 0, y: 0, width: 100, height: 100),
                                    lineWidth: 10, fraction: 1).boundingBox
    #expect(abs(box.minX - 5) < 0.5)
    #expect(abs(box.width - 90) < 0.5)
}

/// Retângulo não-quadrado: o anel continua redondo e centralizado, em vez de
/// virar elipse. O ícone é quadrado, mas o slot do menu bar não é.
@Test func arcStaysCircularInNonSquareRects() {
    let box = GaugeGeometry.arcPath(in: CGRect(x: 0, y: 0, width: 200, height: 100),
                                    lineWidth: 0, fraction: 1).boundingBox
    #expect(abs(box.width - 100) < 0.5)
    #expect(abs(box.height - 100) < 0.5)
    #expect(abs(box.midX - 100) < 0.5)
}
