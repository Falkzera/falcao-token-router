import AppKit
import SwiftUI
import CCUsageCore


/// Ícone de ajuda com a explicação longa atrás dele.
///
/// Tooltip em vez de popover porque `@State` não existe neste toolchain: é
/// macro do SwiftUI e o plugin só acompanha o Xcode. Não se perde nada — o
/// tooltip é o idioma que o macOS já usa para isto.
private struct HelpTip: View {
    let text: LocalizedStringKey

    var body: some View {
        Image(systemName: "info.circle")
            .foregroundStyle(.tertiary)
            .help(text)
    }
}

/// Uma linha de controle: rótulo à esquerda, ajuda à direita.
private struct ControlRow<Content: View>: View {
    let help: LocalizedStringKey
    @ViewBuilder let content: Content

    var body: some View {
        HStack(spacing: 8) {
            content
            Spacer(minLength: 8)
            HelpTip(text: help)
        }
    }
}

/// Chip selecionável. `.button` porque a alternativa seria uma pilha de
/// checkboxes ocupando uma linha cada.
private struct Chip: View {
    let label: LocalizedStringKey
    let isOn: Bool
    let toggle: () -> Void

    var body: some View {
        Toggle(label, isOn: Binding(get: { isOn }, set: { _ in toggle() }))
            .toggleStyle(.button)
            .controlSize(.small)
    }
}

/// A aba do MEDIDOR: plano, sensor, alertas, teto e sistema.
///
/// Já foi a janela de Ajustes inteira, e abria com uma seção "Rodízio de
/// contas" cujo conteúdo era um botão para a janela de Grupos. Com a janela
/// única a seção some: o rodízio é a aba ao lado.
struct SettingsView: View {
    @Bindable var settings: AppSettings
    @Bindable var form: SettingsFormState
    let loginItem: LoginItem
    let alerts: AlertCoordinator
    /// Teto que a calibração automática encontrou — mostrado como referência
    /// para o campo manual não ser preenchido no escuro.
    let calibratedCeiling: UInt64

    var body: some View {
        Form {
            Section("settings.section.plan") {
                Picker("settings.plan.subscription", selection: $settings.plan) {
                    ForEach(Plan.allCases, id: \.self) { plan in
                        Text(verbatim: "\(plan.label) — \(Format.planPrice(plan))").tag(plan)
                    }
                }
                .pickerStyle(.inline)
                if let detected = settings.detectedPlan {
                    Label(settings.planDisagreesWithDetection
                          ? String(format: String(localized: "settings.plan.disagrees.format"),
                                   detected.label)
                          : String(localized: "settings.plan.detected"),
                          systemImage: settings.planDisagreesWithDetection
                          ? "exclamationmark.triangle" : "checkmark.circle")
                        .font(.caption)
                        .foregroundStyle(settings.planDisagreesWithDetection
                                         ? UsageColor.warning : .secondary)
                        .help(settings.planDisagreesWithDetection
                              ? String(localized: "settings.plan.disagrees.help")
                              : String(localized: "settings.plan.detected.help"))
                } else {
                    Label("settings.plan.unknown", systemImage: "questionmark.circle")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            Section("settings.section.usage") {
                ControlRow(help: "settings.live.help") {
                    Toggle("settings.live.toggle", isOn: $settings.liveUsageEnabled)
                }
                Text(settings.liveUsageEnabled ? "settings.live.on" : "settings.live.off")
                    .font(.caption)
                    .foregroundStyle(settings.liveUsageEnabled ? .secondary : UsageColor.warning)
            }

            Section("settings.section.alerts") {
                ControlRow(help: "settings.alerts.thresholds.help") {
                    Toggle("settings.alerts.thresholds.toggle",
                           isOn: $settings.alerts.thresholdsEnabled)
                }

                if settings.alerts.thresholdsEnabled {
                    LabeledContent("settings.alerts.thresholds.label") {
                        HStack(spacing: 6) {
                            ForEach(AlertPreferences.offeredThresholds, id: \.self) { value in
                                Chip(label: "\(value)%",
                                     isOn: settings.alerts.thresholds.contains(value)) {
                                    toggleThreshold(value)
                                }
                            }
                        }
                    }
                    if settings.alerts.thresholds.isEmpty {
                        Label("settings.alerts.thresholds.empty",
                              systemImage: "exclamationmark.triangle")
                            .font(.caption)
                            .foregroundStyle(UsageColor.warning)
                    }
                }

                ControlRow(help: "settings.alerts.reset.help") {
                    Toggle("settings.alerts.reset.toggle",
                           isOn: $settings.alerts.resetEnabled)
                }

                if settings.alerts.anyEnabled {
                    LabeledContent("settings.alerts.windows.label") {
                        HStack(spacing: 6) {
                            Chip(label: "settings.alerts.window.session",
                                 isOn: settings.alerts.windows.contains(.session)) {
                                toggleWindow(.session)
                            }
                            Chip(label: "settings.alerts.window.weekly",
                                 isOn: settings.alerts.windows.contains(.weekly)) {
                                toggleWindow(.weekly)
                            }
                        }
                    }

                    // Alerta sobre cache defasada erraria nos dois sentidos, e o
                    // pior é o silêncio enquanto o usuário estoura. Em vez de
                    // notificar mal, a tela oferece a saída.
                    if !settings.liveUsageEnabled {
                        Label("settings.alerts.needsLive",
                              systemImage: "exclamationmark.triangle")
                            .font(.caption)
                            .foregroundStyle(UsageColor.warning)
                        Button("settings.alerts.enableLive") { settings.liveUsageEnabled = true }
                            .controlSize(.small)
                    }

                    // Uma chave ligada sobre permissão negada é uma chave que mente.
                    if alerts.isDenied {
                        Label("settings.alerts.denied", systemImage: "bell.slash")
                            .font(.caption)
                            .foregroundStyle(UsageColor.critical)
                        Button("settings.alerts.openSystem") {
                            guard let url = URL(string:
                                "x-apple.systempreferences:com.apple.preference.notifications")
                            else { return }
                            NSWorkspace.shared.open(url)
                        }
                        .controlSize(.small)
                    }
                }
            }

            Section("settings.section.ceiling") {
                Picker("", selection: $form.useManualCeiling) {
                    Text("settings.ceiling.calibrate").tag(false)
                    Text("settings.ceiling.manual").tag(true)
                }
                .pickerStyle(.inline)
                .labelsHidden()

                if form.useManualCeiling {
                    TextField("settings.ceiling.tokens", text: $form.ceilingDraft)
                        .onSubmit { form.commit(to: settings) }
                }
                ControlRow(help: "settings.ceiling.help") {
                    Text(String(format: String(localized: "settings.ceiling.calibrated.format"),
                                Format.tokens(calibratedCeiling)))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            Section("settings.section.system") {
                Toggle("settings.system.dock", isOn: $settings.showInDock)
                Text("settings.system.dock.detail")
                    .font(.caption).foregroundStyle(.secondary)
                Toggle("settings.system.loginItem", isOn: Binding(
                    get: { loginItem.isEnabled },
                    set: { loginItem.setEnabled($0) }))
                if let explanation = loginItem.explanation {
                    Text(explanation)
                        .font(.caption)
                        .foregroundStyle(UsageColor.critical)
                }
            }
        }
        .formStyle(.grouped)
        .onAppear {
            loginItem.refresh()
            form.load(from: settings)
            Task { await alerts.refreshAuthorization() }
        }
        .onChange(of: form.useManualCeiling) { _, _ in form.commit(to: settings) }
    }

    private func toggleThreshold(_ value: Int) {
        if settings.alerts.thresholds.contains(value) {
            settings.alerts.thresholds.remove(value)
        } else {
            settings.alerts.thresholds.insert(value)
        }
    }

    private func toggleWindow(_ window: CCUsageCore.Alert.Window) {
        if settings.alerts.windows.contains(window) {
            settings.alerts.windows.remove(window)
        } else {
            settings.alerts.windows.insert(window)
        }
    }
}
