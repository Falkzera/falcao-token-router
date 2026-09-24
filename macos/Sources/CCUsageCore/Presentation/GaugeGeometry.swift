import CoreGraphics
/// Geometria do anel-medidor, sem SwiftUI.
///
/// Mora aqui, e não junto da `Shape`, pela mesma razão que o resto do core mora
/// aqui: assim dá para testar a regra sem instanciar janela. A `Shape` no alvo
/// de UI só desenha o que estas funções decidem.
public enum GaugeGeometry {
    /// Quanto do anel está preenchido, em graus.
    ///
    /// Satura em uma volta: o teto do bloco é calibrado do próprio histórico,
    /// não um limite imposto, então estourá-lo é comum. Um anel que desse a
    /// segunda volta desenharia 110% igual a 10%.
    public static func sweepDegrees(fraction: Double) -> Double {
        clamped(fraction) * 360
    }

    /// Reduz a fração a um dos `steps` desenhos possíveis.
    ///
    /// O label do `MenuBarExtra` só aceita `Image`, então cada estado do anel é
    /// um bitmap rasterizado. Quantizar limita quantos existem — sem isso seria
    /// um render novo a cada atualização de uso.
    ///
    /// O arredondamento é para baixo, com as duas pontas protegidas: anel cheio
    /// só em 100% de verdade, anel vazio só sem consumo nenhum. São os dois
    /// desenhos que afirmam alguma coisa — "bateu o teto" e "não gastei nada" —
    /// e nenhum dos dois pode aparecer por arredondamento.
    public static func quantize(_ fraction: Double, steps: Int) -> Int {
        let fraction = clamped(fraction)
        if fraction <= 0 { return 0 }
        if fraction >= 1 { return steps }
        return max(1, min(steps - 1, Int(fraction * Double(steps))))
    }

    private static func clamped(_ fraction: Double) -> Double {
        min(max(fraction, 0), 1)
    }
}

extension GaugeGeometry {
    /// Onde o anel começa: 12 horas.
    private static let startAngle = -Double.pi / 2

    /// A linha de centro do arco preenchido, em coordenadas y-para-baixo.
    ///
    /// Devolve a linha de centro, não o traço: quem desenha aplica a espessura.
    /// O raio já recua metade dela, então o traço fica dentro de `rect`.
    ///
    /// A convenção y-para-baixo é a do SwiftUI. Um contexto de bitmap do
    /// CoreGraphics é y-para-cima, e desenhar isto lá sem inverter o contexto
    /// espelha o arco — ele passa a girar anti-horário.
    public static func arcPath(in rect: CGRect, lineWidth: CGFloat, fraction: Double) -> CGPath {
        let path = CGMutablePath()
        let sweep = sweepDegrees(fraction: fraction) * .pi / 180
        guard sweep > 0 else { return path }

        let radius = (min(rect.width, rect.height) - lineWidth) / 2
        // `clockwise: false` é o que produz ângulo crescente. Lido em
        // y-para-baixo, ângulo crescente é o giro horário que se quer ver.
        path.addArc(center: CGPoint(x: rect.midX, y: rect.midY),
                    radius: radius,
                    startAngle: startAngle,
                    endAngle: startAngle + sweep,
                    clockwise: false)
        return path
    }

    /// O anel completo, por baixo do arco: mostra o quanto ainda cabe.
    public static func trackPath(in rect: CGRect, lineWidth: CGFloat) -> CGPath {
        arcPath(in: rect, lineWidth: lineWidth, fraction: 1)
    }
}
