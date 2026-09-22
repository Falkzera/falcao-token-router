import AppKit
import SwiftUI
import CCUsageCore

/// Só existe para pegar o `applicationDidFinishLaunching`.
///
/// `NSApp.setActivationPolicy` chamado de `App.init()` **não faz efeito**: nessa
/// hora o `NSApplication` ainda está subindo e a troca é descartada em silêncio
/// (medido — a preferência estava ligada e o app continuava sem Dock). Daqui
/// funciona.
final class AppDelegate: NSObject, NSApplicationDelegate {
    @MainActor var applyDockPolicy: (() -> Void)?

    func applicationDidFinishLaunching(_ notification: Notification) {
        MainActor.assumeIsolated { applyDockPolicy?() }
    }

    /// Clicar no ícone do Dock sem janela aberta reabre a janela, como em
    /// qualquer app de macOS. Sem isto o ícone existiria e não faria nada, que é
    /// pior que não ter ícone.
    func applicationShouldHandleReopen(_ sender: NSApplication,
                                       hasVisibleWindows: Bool) -> Bool {
        if !hasVisibleWindows { NSApp.activate() }
        return true
    }
}

@main
struct FalcaoTokenRouterApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    /// Sem `@State`: no SDK do macOS 26 ele é uma macro do SwiftUI, e o plugin
    /// `SwiftUIMacros` só acompanha o Xcode — não o Command Line Tools.
    ///
    /// Não se perde nada aqui. `@State` governa posse e tempo de vida, e este
    /// app tem exatamente um store que vive enquanto o processo vive. O
    /// redesenho quando `snapshot` muda vem do `@Observable` (cujo plugin de
    /// macro *está* no CLT), que rastreia o acesso à propriedade dentro do
    /// `body` — independentemente de como a referência é guardada.
    @MainActor private static let settings = AppSettings()
    @MainActor private static let store = UsageStore(
        liveUsageEnabled: Self.settings.liveUsageEnabled)
    @MainActor private static let loginItem = LoginItem()
    @MainActor private static let form = SettingsFormState()
    @MainActor private static let alerts = AlertCoordinator(settings: Self.settings)
    /// O motor de grupos: a configuração que o usuário monta na UI, e a rotação.
    @MainActor private static let router = RouterConfigStore()
    /// Qual aba a janela única abre. Store, e não `@State`: quem escolhe é o
    /// painel, de outra cena.
    @MainActor private static let navigation = HomeNavigation()
    init() {
        // A varredura inicial roda em task destacada, então a menu bar aparece
        // imediatamente e o painel preenche quando o disco termina de ser lido.
        Self.store.ceilingOverride = Self.settings.ceilingOverride
        // O gancho antes do start: a primeira reconstrução já estabelece a
        // linha de base da política, em vez de a política começar cega.
        Self.store.onSnapshot = { snapshot in Self.alerts.handle(snapshot) }
        Self.router.routerPath = RouterBinary.path
        // Cura a integração quando o .app mudou de lugar ou de nome: o
        // `shell.sh` e a `statusLine` guardam o caminho absoluto do `router`, e
        // um caminho morto ali faz `claude <grupo>` abrir em silêncio na conta
        // errada. Aqui é o único momento em que se sabe onde o binário está
        // AGORA, então é aqui que se conserta.
        Self.router.healShellIntegration()
        // Duas portas de propósito. O `Task` roda assim que a fila principal
        // gira, o que já é depois de o NSApplication existir; o delegate é a
        // garantia para o caso de o SwiftUI reordenar a subida. As duas são
        // idempotentes — setar a mesma política duas vezes não faz nada.
        delegate.applyDockPolicy = { Self.applyDockPolicy() }
        Task { @MainActor in Self.applyDockPolicy() }
        Self.store.start()
        Self.startRotation()
    }

    /// O laço da rotação automática, no próprio app enquanto ele vive: a cada
    /// três minutos relê o uso do sensor e deixa cada grupo trocar se passou do
    /// limiar. Três minutos porque o `rate_limits` só muda quando há atividade, e
    /// o endpoint de uso tem orçamento próprio — reconsultar mais rápido não traz
    /// número novo, só trabalho.
    @MainActor private static func startRotation() {
        Task {
            while !Task.isCancelled {
                router.refreshUsage()
                router.rotateAll()
                try? await Task.sleep(for: .seconds(180))
            }
        }
    }

    /// Mostra ou esconde o app no Dock.
    ///
    /// O `Info.plist` marca `LSUIElement`, então o app nasce como agente: sem
    /// ícone no Dock, sem ⌘-Tab, sem janela até alguém achar o item na barra de
    /// menus. Isso é o certo para um widget — e é um beco sem saída quando a
    /// barra está cheia: com dezenove itens numa tela com notch, o macOS
    /// **esconde o que não cabe sem avisar**, e o app fica rodando sem nenhuma
    /// superfície por onde ser aberto.
    ///
    /// Trocar a política em tempo de execução é o que devolve a saída. Com
    /// `.regular` o app vira um app comum, e o ícone do Dock passa a ser a porta
    /// que a barra não estava sendo.
    @MainActor private static func applyDockPolicy() {
        // `NSApplication.shared`, e não `NSApp`. O segundo é um global
        // implicitamente desembrulhado que **só existe depois** que o
        // NSApplication sobe: chamá-lo de `App.init()` derrubou o app inteiro no
        // lançamento, com "Unexpectedly found nil while implicitly unwrapping an
        // Optional" (crash real, 22/09/2026). `shared` cria a instância se
        // preciso e é seguro em qualquer momento.
        NSApplication.shared.setActivationPolicy(settings.showInDock ? .regular : .accessory)
    }

    /// A janela abre sozinha no lançamento quando não há o que mostrar na barra
    /// ainda (primeira execução, nenhum grupo criado) ou quando o usuário pediu
    /// o Dock — aí ele espera um app comum, e app comum abre janela.
    ///
    /// `static let` porque a decisão é do lançamento: reavaliá-la depois faria a
    /// janela reaparecer sozinha no meio do uso.
    @MainActor private static let presentAtLaunch =
        router.config.groups.isEmpty || settings.showInDock

    /// Liga a preferência ao store num lugar só.
    ///
    /// O `onChange` abaixo vive na cena de Ajustes, e cena só existe com a
    /// janela aberta: ligar o ao vivo pelo painel, sem nunca ter aberto os
    /// Ajustes, gravaria a preferência e deixaria o store sem saber. O número
    /// continuaria vindo do cache com a tela dizendo que está ao vivo.
    @MainActor private static func setLiveUsage(_ enabled: Bool) {
        settings.liveUsageEnabled = enabled
        store.liveUsageEnabled = enabled
    }

    var body: some Scene {
        MenuBarExtra {
            UsagePanel(snapshot: Self.store.snapshot,
                       plan: Self.settings.plan,
                       canEnableLive: !Self.settings.liveUsageEnabled,
                       onEnableLive: { Self.setLiveUsage(true) },
                       router: Self.router,
                       navigation: Self.navigation)
                .onAppear {
                    Self.store.panelDidOpen()
                    // Abrir o painel é o momento em que o número velho mais
                    // incomoda; uma releitura aqui custa pouco e chega a tempo.
                    Self.router.refreshUsage()
                }
        } label: {
            MenuBarLabel(snapshot: Self.store.snapshot, router: Self.router)
        }
        .menuBarExtraStyle(.window)

        // A janela única: rodízio e medidor em abas. Eram duas cenas — um
        // `Window` de grupos e uma `Settings` — e cada uma existia em parte
        // para apontar para a outra.
        Window("home.title", id: HomeWindowID.value) {
            HomeWindow(navigation: Self.navigation,
                       settings: Self.settings,
                       form: Self.form,
                       loginItem: Self.loginItem,
                       alerts: Self.alerts,
                       router: Self.router,
                       calibratedCeiling: Self.store.snapshot.calibratedBlockCeiling)
                // O store é a única fonte do denominador; as settings só
                // publicam a intenção do usuário e esta ponte a aplica.
                .onChange(of: Self.settings.ceilingOverride) { _, override in
                    Self.store.ceilingOverride = override
                }
                .onChange(of: Self.settings.liveUsageEnabled) { _, enabled in
                    Self.setLiveUsage(enabled)
                }
                .onChange(of: Self.settings.alerts) { _, preferences in
                    Self.alerts.alertsEnabledChanged(to: preferences.anyEnabled)
                }
                .onChange(of: Self.settings.showInDock) { _, _ in
                    Self.applyDockPolicy()
                }
        }
        .windowResizability(.contentSize)
        .defaultLaunchBehavior(Self.presentAtLaunch ? .presented : .suppressed)
        .commands {
            // A cena `Settings` dava "Ajustes…" e o ⌘, de graça; sem ela, o
            // atalho ficaria mudo.
            CommandGroup(replacing: .appSettings) {
                SettingsCommand(navigation: Self.navigation)
            }
        }
    }
}
