import SwiftUI
import CCUsageCore

/// A seção dos Ajustes que escolhe o que a status line das sessões de grupo
/// mostra.
///
/// A prévia é desenhada pelo MESMO código da sessão (`StatusLineView.render`
/// com a mesma `StatusLineChoice`), sobre um fundo escuro de terminal: as cores
/// da linha são feitas para ele, e mostrá-las sobre o fundo do sistema daria
/// uma prévia que mente sobre o resultado.
struct StatusLineSection: View {
    @Bindable var store: RouterConfigStore

    /// A sessão de exemplo é montada uma vez: os resets dela são relativos a
    /// "agora", e remontar a cada toque faria os horários andarem na tela.
    private let sample = StatusLineSession.sample()

    private var choice: StatusLineChoice { store.statusLineChoice }

    var body: some View {
        Section("settings.statusline.section") {
            Text("settings.statusline.explain")
                .font(.callout)
                .foregroundStyle(.secondary)

            preview

            ForEach(StatusLineChoice.Item.allCases, id: \.self) { item in
                Toggle(LocalizedStringKey(Self.key(item)), isOn: Binding(
                    get: { choice.shows(item) },
                    set: { store.setStatusLineItem(item, shown: $0) }))
            }

            if !choice.hiddenItems.isEmpty {
                Button("settings.statusline.restore") { store.restoreStatusLine() }
            }
        }
    }

    private var preview: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            Text(ANSIText.attributed(choice.apply(sample).render(
                .init(trueColor: true,
                      portuguese: Locale.current.language.languageCode?.identifier == "pt",
                      // Fase fixa: uma prévia que pisca tira a atenção do que
                      // ela existe para mostrar.
                      phase: 0))))
                .font(.system(.callout, design: .monospaced))
                .textSelection(.enabled)
                .padding(.horizontal, 10)
                .padding(.vertical, 8)
                .fixedSize(horizontal: true, vertical: false)
        }
        .background(Color(red: 0.11, green: 0.11, blue: 0.12))
        .clipShape(RoundedRectangle(cornerRadius: 6))
        .overlay(alignment: .leading) {
            if choice.hiddenItems.count == StatusLineChoice.Item.allCases.count {
                Text("settings.statusline.empty")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 10)
            }
        }
    }

    /// A chave do catálogo de cada item. Explícito e não derivado do `rawValue`:
    /// o `check-strings.sh` precisa VER a chave no fonte para saber que ela é
    /// usada, e uma chave montada em tempo de execução seria apagada como órfã.
    private static func key(_ item: StatusLineChoice.Item) -> String {
        switch item {
        case .group:    "settings.statusline.item.group"
        case .model:    "settings.statusline.item.model"
        case .effort:   "settings.statusline.item.effort"
        case .place:    "settings.statusline.item.place"
        case .context:  "settings.statusline.item.context"
        case .fiveHour: "settings.statusline.item.fiveHour"
        case .sevenDay: "settings.statusline.item.sevenDay"
        case .resets:   "settings.statusline.item.resets"
        case .cost:     "settings.statusline.item.cost"
        case .email:    "settings.statusline.item.email"
        }
    }
}
