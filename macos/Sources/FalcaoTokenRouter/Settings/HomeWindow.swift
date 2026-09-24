import AppKit
import Observation
import SwiftUI
import CCUsageCore

/// As duas metades do app fora do painel: o rodízio (o produto) e o medidor.
enum HomeTab: Hashable {
    case groups
    case meter
}

/// Qual aba a janela mostra.
///
/// Fora da view de propósito: quem escolhe a aba é o painel do menu bar, que
/// vive em OUTRA cena. Como `@State` da janela, a escolha se perderia — a
/// janela é recriada, e o clique em "Ajustes" cairia em Grupos.
@MainActor
@Observable
final class HomeNavigation {
    var tab: HomeTab = .groups
}

/// A janela única do app: rodízio e ajustes do medidor em abas.
///
/// Eram duas janelas separadas até 28/08/2026, e a divisão não se sustentava.
/// A de Ajustes abria com uma seção — a primeira da tela — cujo conteúdo
/// inteiro era um botão "Abrir Grupos"; a de Grupos tinha uma engrenagem de
/// volta para os Ajustes. Ponte de mão dupla entre duas telas é o sintoma de
/// que são uma só: o usuário não escolhia entre elas, ele saltava entre elas.
///
/// O ganho não é estético. Rodízio e medidor decidem juntos: o limiar que faz
/// a conta trocar (Grupos) só significa alguma coisa sobre o número que o
/// sensor mede (Medidor), e o sensor desligado deixa o rodízio decidindo com o
/// cache — que já foi medido 12 horas atrasado. Separados em janelas, esse par
/// nunca aparecia na mesma tela.
struct HomeWindow: View {
    @Bindable var navigation: HomeNavigation
    @Bindable var settings: AppSettings
    let form: SettingsFormState
    let loginItem: LoginItem
    let alerts: AlertCoordinator
    let router: RouterConfigStore
    let calibratedCeiling: UInt64

    var body: some View {
        TabView(selection: $navigation.tab) {
            GroupsView(store: router)
                .tabItem { Label("home.tab.groups", systemImage: "rectangle.stack") }
                .tag(HomeTab.groups)

            SettingsView(settings: settings, router: router, form: form, loginItem: loginItem,
                         alerts: alerts, calibratedCeiling: calibratedCeiling)
                .tabItem { Label("home.tab.meter", systemImage: "gauge.with.needle") }
                .tag(HomeTab.meter)
        }
        // Tamanho fixo, e não `.contentSize` livre: as duas abas têm alturas
        // naturais diferentes e a janela pularia de tamanho a cada troca.
        .frame(width: 520, height: 620)
        .onAppear { NSApp.activate() }
    }
}

/// O item de menu que ⌘, dispara.
///
/// Existe porque a cena `Settings` saiu: era ela que dava o "Ajustes…" e o
/// atalho de graça. Sem esta substituição, ⌘, ficaria mudo — e um atalho que
/// todo app de macOS tem, morto, é pior que a janela que ele abria.
struct SettingsCommand: View {
    let navigation: HomeNavigation
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        Button("home.menu.settings") {
            navigation.tab = .meter
            openWindow(id: HomeWindowID.value)
            NSApp.activate()
        }
        .keyboardShortcut(",", modifiers: .command)
    }
}

/// O id da cena, num lugar só: ele é escrito no `App` e lido por quem abre a
/// janela (painel e menu), e literal repetido em três arquivos falha em
/// silêncio — `openWindow` com id inexistente não abre nada e não avisa.
enum HomeWindowID {
    static let value = "home"
}
