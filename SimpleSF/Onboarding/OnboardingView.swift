import SwiftUI

// Ref: FT-SSF-007, FT-SSF-015
@MainActor
struct OnboardingView: View {
    @ObservedObject private var keychain = KeychainService.shared
    @ObservedObject private var llm = LLMService.shared
    @ObservedObject var mlx = MLXService.shared
    @ObservedObject var ollama = OllamaService.shared
    @ObservedObject private var appState = AppState.shared
    @ObservedObject private var l10n = L10n.shared
    @State private var keys: [LLMProvider: String] = [:]
    @State private var testing: LLMProvider? = nil
    @State private var testResults: [LLMProvider: Bool] = [:]
    @State private var modelOverrides: [LLMProvider: String] = [:]

    var isSelected: LLMProvider? { appState.selectedProvider }

    var body: some View {
        VStack(spacing: 0) {
            IHMContextHeader(context: .onboarding)

            // Header
            HStack {
                Image(systemName: "gearshape.fill").foregroundColor(SF.Colors.purple)
                Text(l10n.t(.settingsTitle))
                    .font(.title2.bold())
                    .foregroundColor(SF.Colors.textPrimary)
                Spacer()
                activeBadge
            }
            .padding()

            Divider().background(SF.Colors.border)

            ScrollView {
                VStack(spacing: 20) {
                    languageSection
                    activeModelBanner
                    localLLMSection
                    cloudSection
                }
                .padding()
            }

            Divider().background(SF.Colors.border)

            HStack(spacing: 16) {
                Image(systemName: "lock.shield.fill")
                    .foregroundColor(.green)
                VStack(alignment: .leading, spacing: 2) {
                    Text(l10n.t(.privacyLocal))
                        .font(.caption)
                        .foregroundColor(SF.Colors.textSecondary)
                    Text(l10n.t(.privacyCloud))
                        .font(.caption)
                        .foregroundColor(SF.Colors.textSecondary)
                }
            }
            .padding()
        }
        .background(SF.Colors.bgPrimary)
        .onAppear {
            for p in LLMProvider.cloudProviders {
                if let k = keychain.key(for: p) { keys[p] = k }
            }
            // Load saved model overrides
            for p in LLMProvider.allCases {
                let saved = UserDefaults.standard.string(forKey: "sf_model_\(p.rawValue)") ?? ""
                if !saved.isEmpty { modelOverrides[p] = saved }
            }
            mlx.scanModels()
            Task { await ollama.refresh() }
        }
    }

    // MARK: - Active Model Banner

    private var activeModelBanner: some View {
        VStack(spacing: 12) {
            HStack(spacing: 14) {
                ZStack {
                    Circle()
                        .fill(llm.activeProvider != nil ? Color.green.opacity(0.15) : Color.orange.opacity(0.15))
                        .frame(width: 44, height: 44)
                    Image(systemName: llm.activeProvider != nil ? "cpu.fill" : "exclamationmark.triangle.fill")
                        .font(.system(size: 20))
                        .foregroundColor(llm.activeProvider != nil ? .green : .orange)
                }

                VStack(alignment: .leading, spacing: 3) {
                    if let prov = llm.activeProvider {
                        Text(activeProviderLabel(prov))
                            .font(.system(size: 12, weight: .medium))
                            .foregroundColor(SF.Colors.textSecondary)
                        Text(activeModelDescription(prov))
                            .font(.system(size: 15, weight: .bold, design: .monospaced))
                            .foregroundColor(SF.Colors.textPrimary)
                    } else {
                        Text(l10n.t(.settingsNoModelActive))
                            .font(.system(size: 12, weight: .medium))
                            .foregroundColor(SF.Colors.textSecondary)
                        Text(l10n.t(.settingsSelectProvider))
                            .font(.system(size: 15, weight: .bold))
                            .foregroundColor(.orange)
                    }
                }

                Spacer()

                if isSelected != nil {
                    Button(action: { appState.clearActiveProvider() }) {
                        Label(l10n.t(.sidebarAutoDetect), systemImage: "arrow.counterclockwise")
                            .font(.caption)
                    }
                    .buttonStyle(.bordered)
                    .tint(SF.Colors.textMuted)
                    .controlSize(.small)
                    .help(l10n.t(.sidebarAutoDetectHelp))
                }
            }

            if llm.activeProvider == nil {
                Text(l10n.t(.settingsConfigureHint))
                    .font(.caption)
                    .foregroundColor(SF.Colors.textSecondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .padding(16)
        .background(
            RoundedRectangle(cornerRadius: 12)
                .fill(SF.Colors.bgSecondary)
                .overlay(
                    RoundedRectangle(cornerRadius: 12)
                        .stroke(llm.activeProvider != nil ? Color.green.opacity(0.3) : Color.orange.opacity(0.3), lineWidth: 1.5)
                )
        )
    }

    private func activeModelDescription(_ prov: LLMProvider) -> String {
        switch prov {
        case .mlx:
            let name = MLXService.shared.activeModel?.name ?? "loading…"
            let short = name.split(separator: "/").last.map(String.init) ?? name
            return short
        case .ollama:
            return OllamaService.shared.activeModel?.name ?? "loading…"
        default:
            let model = modelOverrides[prov] ?? prov.defaultModel
            return model
        }
    }

    private func activeProviderLabel(_ prov: LLMProvider) -> String {
        if prov.isLocal { return "🖥 \(prov.displayName) (local)" }
        return "☁️ \(prov.displayName) (cloud)"
    }

    // MARK: - Language Section

    static let languages: [(code: String, name: String)] = [
        ("fr", "Francais"), ("en", "English"), ("es", "Espanol"), ("de", "Deutsch"),
        ("it", "Italiano"), ("pt", "Portugues"), ("ja", "Japanese"), ("ko", "Korean"),
        ("zh", "Chinese"), ("ar", "Arabic"), ("ru", "Russian"), ("nl", "Dutch")
    ]

    private var languageSection: some View {
        LanguagePickerView()
    }

    // MARK: - Active provider badge (header)

    @ViewBuilder
    private var activeBadge: some View {
        if let prov = llm.activeProvider {
            HStack(spacing: 6) {
                Circle().fill(Color.green).frame(width: 8, height: 8)
                Text(llm.activeDisplayName)
                    .font(.caption.bold())
            }
            .foregroundColor(.green)
        } else {
            Label(l10n.t(.settingsNoProvider), systemImage: "exclamationmark.triangle.fill")
                .font(.caption)
                .foregroundColor(.orange)
        }
    }

    // Local LLM section: see OnboardingStepView.swift

    // MARK: - Cloud Section

    private var cloudSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Image(systemName: "cloud.fill")
                    .foregroundColor(.blue)
                Text(l10n.t(.settingsCloudProviders))
                    .font(.headline)
                    .foregroundColor(SF.Colors.textPrimary)
                Spacer()
                Text(l10n.t(.settingsCloudHint))
                    .font(.caption2)
                    .foregroundColor(SF.Colors.textSecondary)
            }

            ForEach(LLMProvider.cloudProviders, id: \.self) { provider in
                CloudProviderCard(
                    provider: provider,
                    storedKey: keychain.key(for: provider),
                    draftKey: Binding(
                        get: { keys[provider] ?? "" },
                        set: { keys[provider] = $0 }
                    ),
                    modelOverride: Binding(
                        get: { modelOverrides[provider] ?? "" },
                        set: { modelOverrides[provider] = $0 }
                    ),
                    isSelected: isSelected == provider,
                    isTesting: testing == provider,
                    testResult: testResults[provider],
                    onSave: { save(provider: provider) },
                    onTest: { Task { await test(provider: provider) } },
                    onDelete: { keychain.delete(for: provider) },
                    onUse: { activate(provider) }
                )
            }
        }
        .padding()
        .background(SF.Colors.bgSecondary)
        .cornerRadius(12)
    }

    // MARK: - Shared helpers

    @ViewBuilder
    func useButton(_ provider: LLMProvider, available: Bool) -> some View {
        if isSelected == provider {
            Label(l10n.t(.statusActive), systemImage: "checkmark.circle.fill")
                .font(.caption.bold())
                .foregroundColor(.green)
        } else if available {
            Button(action: { activate(provider) }) {
                Text(l10n.t(.actionUse))
                    .font(.caption.bold())
            }
            .buttonStyle(.borderedProminent)
            .tint(SF.Colors.purple)
            .controlSize(.mini)
        }
    }

    func cardBg(_ provider: LLMProvider) -> Color {
        isSelected == provider
            ? SF.Colors.purple.opacity(0.08)
            : SF.Colors.bgTertiary.opacity(0.5)
    }

    @ViewBuilder
    func selectionBorder(_ provider: LLMProvider) -> some View {
        if isSelected == provider {
            RoundedRectangle(cornerRadius: 8)
                .stroke(SF.Colors.purple.opacity(0.5), lineWidth: 1.5)
        }
    }

    private func activate(_ provider: LLMProvider) {
        let model = modelOverrides[provider] ?? ""
        appState.setActiveProvider(provider, model: model.isEmpty ? nil : model)
    }

    // MARK: - Actions

    private func save(provider: LLMProvider) {
        let key = (keys[provider] ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        keychain.save(key: key, for: provider)
    }

    private func test(provider: LLMProvider) async {
        let key = (keys[provider] ?? keychain.key(for: provider) ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        guard !key.isEmpty else { return }
        testing = provider
        let result = await llm.testConnection(provider: provider, key: key)
        testResults[provider] = result
        testing = nil
    }
}

