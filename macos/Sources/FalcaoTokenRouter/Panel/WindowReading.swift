import SwiftUI
import CCUsageCore

/// Uma janela do `rate_limits` numa linha de conta: rótulo e valor.
///
/// As DUAS aparecem, sempre. Mostrar só a que decide a rotação resolvia a
/// ambiguidade do número solto e criava outra: some da tela justamente a
/// pergunta que se faz primeiro — "quanto me resta agora?". A janela que manda
/// se distingue pelo peso e pela cor, não por ser a única visível.
struct WindowReading: View {
    /// Onde a leitura aparece — e, por isso, se o rótulo vem junto dela.
    ///
    /// No painel as contas formam uma TABELA. Repetir "5h" e "7d" em cada linha
    /// somava seis repetições do mesmo par de palavras e, pior, punha os
    /// números em posições diferentes conforme o tamanho do que vinha antes:
    /// nada alinhava de conta para conta. Ali o rótulo sobe UMA vez para o
    /// cabeçalho da coluna e a leitura vira só o número, de largura fixa.
    ///
    /// Na tela de Grupos cada conta é um cartão solto, sem coluna acima para
    /// ancorar o número: lá o rótulo continua ao lado, e é o mesmo código.
    enum Layout {
        case labeled
        case column
    }

    let label: LocalizedStringKey
    let fraction: Double?
    /// Esta é a janela que dita o número da rotação (o maior das duas).
    let isBound: Bool
    /// Amostra velha: o valor continua, mas sem a cor que sugere frescor.
    let isStale: Bool
    var layout: Layout = .labeled

    var body: some View {
        HStack(spacing: 2) {
            if layout == .labeled {
                Text(label)
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
            if let fraction {
                Text(Format.percent(fraction))
                    .font(.caption2.monospacedDigit())
                    .fontWeight(isBound ? .semibold : .regular)
                    .foregroundStyle(tint(fraction))
                    .frame(width: AccountsLayout.number, alignment: .trailing)
            } else {
                // Janela sem dado ou já expirada: um traço é honesto, um "0%"
                // seria mentira confortável.
                // Mais apagado que um número de verdade: repetido na coluna
                // inteira (é o normal em conta ociosa), com o mesmo peso dos
                // valores ele virava a coisa mais visível da tabela.
                Text("panel.accounts.window.absent")
                    .font(.caption2)
                    .foregroundStyle(.quaternary)
                    .frame(width: AccountsLayout.number, alignment: .trailing)
            }
        }
    }

    /// Só a janela que decide ganha a cor de severidade: com as duas coloridas,
    /// nada indicaria qual delas dispara a troca.
    private func tint(_ fraction: Double) -> AnyShapeStyle {
        guard isBound, !isStale else { return AnyShapeStyle(.secondary) }
        return AnyShapeStyle(UsageColor.bar(fraction))
    }
}
