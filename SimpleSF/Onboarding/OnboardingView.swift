import SwiftUI

@MainActor
struct OnboardingView: View {
    @ObservedObject private var keychain = KeychainService.shared
    @ObservedObject private var llm = LLMService.shared
    @ObservedObject private var mlx = MLXService.shared
    @ObservedObject private var ollama = OllamaService.shared
    @ObservedObject private var appState = AppState.shared
    @State private var keys: [LLMProvider: String] = [:]
    @State private var testing: LLMProvider? = nil
    @State private var testResults: [LLMProvider: Bool] = [:]
    @State private var modelOverrides: [LLMProvider: String] = [:]

    private var isSelected: LLMProvider? { appState.selectedProvider }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Image(systemName: "gearshape.fill").foregroundColor(SF.Colors.purple)
                Text("Settings")
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
                    Text("Local models run 100% on your Mac. No data leaves your machine.")
                        .font(.caption)
                        .foregroundColor(SF.Colors.textSecondary)
                    Text("Cloud API keys stored in macOS Keychain — sent only to the provider.")
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
                    Text("Active Model")
                        .font(.system(size: 12, weight: .medium))
                        .foregroundColor(SF.Colors.textSecondary)
                    if let prov = llm.activeProvider {
                        Text(activeModelDescription(prov))
                            .font(.system(size: 16, weight: .bold))
                            .foregroundColor(SF.Colors.textPrimary)
                    } else {
                        Text("No model configured")
                            .font(.system(size: 16, weight: .bold))
                            .foregroundColor(.orange)
                    }
                }

                Spacer()

                if isSelected != nil {
                    Button(action: { appState.clearActiveProvider() }) {
                        Label("Auto", systemImage: "arrow.counterclockwise")
                            .font(.caption)
                    }
                    .buttonStyle(.bordered)
                    .tint(SF.Colors.textMuted)
                    .controlSize(.small)
                    .help("Switch back to auto-detect")
                }
            }

            if llm.activeProvider == nil {
                Text("Configure a local or cloud provider below, then click \"Use\" to activate it.")
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
            let name = MLXService.shared.activeModel?.name ?? "MLX"
            return "MLX · \(name.split(separator: "/").last.map(String.init) ?? name)"
        case .ollama:
            return "Ollama · \(OllamaService.shared.activeModel?.name ?? "?")"
        default:
            let model = modelOverrides[prov] ?? prov.defaultModel
            return "\(prov.displayName) · \(model)"
        }
    }

    // MARK: - Language Section

    static let languages: [(code: String, name: String)] = [
        ("fr", "Francais"), ("en", "English"), ("es", "Espanol"), ("de", "Deutsch"),
        ("it", "Italiano"), ("pt", "Portugues"), ("ja", "Japanese"), ("ko", "Korean"),
        ("zh", "Chinese"), ("ar", "Arabic"), ("ru", "Russian"), ("nl", "Dutch")
    ]

    private var languageSection: some View {
        HStack(spacing: 12) {
            Image(systemName: "globe")
                .foregroundColor(SF.Colors.purple)
            Text("Language")
                .font(.headline)
                .foregroundColor(SF.Colors.textPrimary)
            Picker("", selection: $appState.selectedLang) {
                ForEach(Self.languages, id: \.code) { lang in
                    Text(lang.name).tag(lang.code)
                }
            }
            .labelsHidden()
            .frame(maxWidth: 200)
            .onChange(of: appState.selectedLang) { newValue in
                appState.setLanguage(newValue)
            }
            Spacer()
            Text("Jarvis responds in this language")
                .font(.caption2)
                .foregroundColor(SF.Colors.textSecondary)
        }
        .padding()
        .background(SF.Colors.bgSecondary)
        .cornerRadius(12)
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
            Label("No provider", systemImage: "exclamationmark.triangle.fill")
                .font(.caption)
                .foregroundColor(.orange)
        }
    }

    // MARK: - Local LLM Section

    private var localLLMSection: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Image(systemName: "desktopcomputer")
                    .foregroundColor(SF.Colors.purple)
                Text("Local LLM")
                    .font(.headline)
                    .foregroundColor(SF.Colors.textPrimary)
                Spacer()
                Text("Zero network — runs on Apple Silicon")
                    .font(.caption2)
                    .foregroundColor(SF.Colors.textSecondary)
            }

            ollamaCard
            mlxCard
        }
        .padding()
        .background(SF.Colors.bgSecondary)
        .cornerRadius(12)
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(
                    (ollama.isRunning || mlx.isRunning) ? Color.green.opacity(0.4) : SF.Colors.border,
                    lineWidth: 1.5
                )
        )
    }

    // MARK: - Ollama Card

    private var ollamaCard: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Ollama")
                    .font(.subheadline.bold())
                    .foregroundColor(SF.Colors.textPrimary)
                if ollama.isRunning {
                    Label("Running", systemImage: "circle.fill")
                        .font(.caption2)
                        .foregroundColor(.green)
                } else {
                    Label("Stopped", systemImage: "circle")
                        .font(.caption2)
                        .foregroundColor(SF.Colors.textMuted)
                }
                Spacer()
                if ollama.isRunning {
                    useButton(.ollama, available: true)
                }
                Button(action: { Task { await ollama.refresh() } }) {
                    Image(systemName: "arrow.clockwise")
                        .font(.caption)
                }
                .buttonStyle(.plain)
                .foregroundColor(SF.Colors.purple)
            }

            if ollama.isRunning {
                if !ollama.availableModels.isEmpty {
                    HStack {
                        Text("Model:")
                            .font(.callout)
                            .foregroundColor(SF.Colors.textSecondary)
                        Picker("", selection: $ollama.activeModel) {
                            ForEach(ollama.availableModels) { model in
                                HStack {
                                    Text(model.name)
                                    Text("(\(model.size))")
                                        .foregroundColor(.secondary)
                                }
                                .tag(model as OllamaService.OllamaModel?)
                            }
                        }
                        .labelsHidden()
                        .frame(maxWidth: 300)
                    }
                } else {
                    Text("No models installed. Run: ollama pull qwen3:14b")
                        .font(.caption)
                        .foregroundColor(SF.Colors.textSecondary)
                }

                HStack(spacing: 12) {
                    Button(action: { ollama.stop() }) {
                        Label("Stop", systemImage: "stop.circle.fill")
                    }
                    .buttonStyle(.bordered)
                    .tint(.red)
                    .controlSize(.small)

                    Text("Port \(ollama.port)")
                        .font(.caption.monospaced())
                        .foregroundColor(SF.Colors.textSecondary)
                }
            } else {
                HStack(spacing: 12) {
                    Button(action: { ollama.start() }) {
                        Label("Start Ollama", systemImage: "play.circle.fill")
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(SF.Colors.purple)
                    .controlSize(.small)

                    Text("or run: ollama serve")
                        .font(.caption2)
                        .foregroundColor(SF.Colors.textSecondary)
                }
            }
        }
        .padding(12)
        .background(cardBg(.ollama))
        .cornerRadius(8)
        .overlay(selectionBorder(.ollama))
    }

    // MARK: - MLX Card

    private var mlxCard: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("MLX")
                    .font(.subheadline.bold())
                    .foregroundColor(SF.Colors.textPrimary)
                mlxStatusBadge
                Spacer()
                if mlx.isRunning {
                    useButton(.mlx, available: true)
                }
                Button(action: { mlx.scanModels() }) {
                    Image(systemName: "arrow.clockwise")
                        .font(.caption)
                }
                .buttonStyle(.plain)
                .foregroundColor(SF.Colors.purple)
            }

            if !mlx.availableModels.isEmpty {
                HStack {
                    Text("Model:")
                        .font(.callout)
                        .foregroundColor(SF.Colors.textSecondary)
                    Picker("", selection: $mlx.activeModel) {
                        ForEach(mlx.availableModels) { model in
                            HStack {
                                Text(model.name)
                                if !model.modelType.isEmpty {
                                    Text(model.modelType)
                                        .foregroundColor(.secondary)
                                }
                                Text(String(format: "%.1f GB", model.sizeGB))
                                    .foregroundColor(.secondary)
                            }
                            .tag(model as MLXService.MLXModel?)
                        }
                    }
                    .labelsHidden()
                    .frame(maxWidth: 300)
                }
            } else {
                Text("No MLX models in ~/.cache/huggingface/hub/ or ~/.cache/mlx-models/")
                    .font(.caption)
                    .foregroundColor(SF.Colors.textSecondary)
            }

            HStack(spacing: 12) {
                if mlx.isRunning {
                    Button(action: { mlx.stop() }) {
                        Label("Stop", systemImage: "stop.circle.fill")
                    }
                    .buttonStyle(.bordered)
                    .tint(.red)
                    .controlSize(.small)

                    Text("Port \(mlx.port)")
                        .font(.caption.monospaced())
                        .foregroundColor(SF.Colors.textSecondary)
                } else {
                    Button(action: {
                        mlx.start()
                        Task {
                            try? await Task.sleep(nanoseconds: 5_000_000_000)
                            if mlx.isRunning { SFBridge.shared.syncLLMConfig() }
                        }
                    }) {
                        Label("Start Server", systemImage: "play.circle.fill")
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(SF.Colors.purple)
                    .controlSize(.small)
                    .disabled(mlx.activeModel == nil)
                }
            }

            if !mlx.logLines.isEmpty {
                ScrollView(.vertical) {
                    VStack(alignment: .leading, spacing: 2) {
                        ForEach(Array(mlx.logLines.suffix(3).enumerated()), id: \.offset) { _, line in
                            Text(line)
                                .font(.caption2.monospaced())
                                .foregroundColor(SF.Colors.textMuted)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                .frame(maxHeight: 40)
            }
        }
        .padding(12)
        .background(cardBg(.mlx))
        .cornerRadius(8)
        .overlay(selectionBorder(.mlx))
    }

    @ViewBuilder
    private var mlxStatusBadge: some View {
        switch mlx.state {
        case .stopped:
            Label("Stopped", systemImage: "circle")
                .font(.caption2)
                .foregroundColor(SF.Colors.textMuted)
        case .starting:
            HStack(spacing: 4) {
                ProgressView().scaleEffect(0.5)
                Text("Starting...")
                    .font(.caption2)
                    .foregroundColor(.orange)
            }
        case .running:
            Label("Running", systemImage: "circle.fill")
                .font(.caption2)
                .foregroundColor(.green)
        case .error(let msg):
            Label(msg, systemImage: "exclamationmark.circle.fill")
                .font(.caption2)
                .foregroundColor(.red)
                .lineLimit(1)
        }
    }

    // MARK: - Cloud Section

    private var cloudSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Image(systemName: "cloud.fill")
                    .foregroundColor(.blue)
                Text("Cloud Providers")
                    .font(.headline)
                    .foregroundColor(SF.Colors.textPrimary)
                Spacer()
                Text("Requires API key")
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
    private func useButton(_ provider: LLMProvider, available: Bool) -> some View {
        if isSelected == provider {
            Label("Active", systemImage: "checkmark.circle.fill")
                .font(.caption.bold())
                .foregroundColor(.green)
        } else if available {
            Button(action: { activate(provider) }) {
                Text("Use")
                    .font(.caption.bold())
            }
            .buttonStyle(.borderedProminent)
            .tint(SF.Colors.purple)
            .controlSize(.mini)
        }
    }

    private func cardBg(_ provider: LLMProvider) -> Color {
        isSelected == provider
            ? SF.Colors.purple.opacity(0.08)
            : SF.Colors.bgTertiary.opacity(0.5)
    }

    @ViewBuilder
    private func selectionBorder(_ provider: LLMProvider) -> some View {
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

// MARK: - Cloud Provider Card

struct CloudProviderCard: View {
    let provider: LLMProvider
    let storedKey: String?
    @Binding var draftKey: String
    @Binding var modelOverride: String
    let isSelected: Bool
    let isTesting: Bool
    let testResult: Bool?
    let onSave: () -> Void
    let onTest: () -> Void
    let onDelete: () -> Void
    let onUse: () -> Void

    @State private var expanded = false

    private var hasKey: Bool { storedKey != nil && !storedKey!.isEmpty }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Row header
            Button(action: { withAnimation(.spring(response: 0.3)) { expanded.toggle() } }) {
                HStack(spacing: 10) {
                    // Selection indicator
                    if isSelected {
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundColor(.green)
                            .font(.system(size: 14))
                    } else {
                        Circle()
                            .fill(hasKey ? Color.green : SF.Colors.textMuted.opacity(0.3))
                            .frame(width: 8, height: 8)
                    }

                    Text(provider.displayName)
                        .font(.headline)
                        .foregroundColor(SF.Colors.textPrimary)

                    Text("· \(modelOverride.isEmpty ? provider.defaultModel : modelOverride)")
                        .font(.caption)
                        .foregroundColor(SF.Colors.textSecondary)

                    Spacer()

                    if hasKey {
                        if isTesting {
                            ProgressView().scaleEffect(0.7)
                        } else if let ok = testResult {
                            Image(systemName: ok ? "checkmark.circle.fill" : "xmark.circle.fill")
                                .foregroundColor(ok ? .green : .red)
                        }

                        if !isSelected {
                            Button(action: onUse) {
                                Text("Use")
                                    .font(.caption.bold())
                            }
                            .buttonStyle(.borderedProminent)
                            .tint(SF.Colors.purple)
                            .controlSize(.mini)
                        } else {
                            Label("Active", systemImage: "checkmark.circle.fill")
                                .font(.caption.bold())
                                .foregroundColor(.green)
                        }
                    }

                    Image(systemName: expanded ? "chevron.up" : "chevron.down")
                        .font(.caption)
                        .foregroundColor(SF.Colors.textMuted)
                }
                .padding(12)
            }
            .buttonStyle(.plain)

            if expanded {
                Divider().background(SF.Colors.border)
                VStack(alignment: .leading, spacing: 10) {
                    // Model override
                    HStack {
                        Text("Model:")
                            .font(.callout)
                            .foregroundColor(SF.Colors.textSecondary)
                        TextField(provider.defaultModel, text: $modelOverride)
                            .textFieldStyle(.roundedBorder)
                            .frame(maxWidth: 250)
                            .onChange(of: modelOverride) { newValue in
                                UserDefaults.standard.set(newValue, forKey: "sf_model_\(provider.rawValue)")
                                if isSelected {
                                    AppState.shared.setActiveProvider(provider, model: newValue.isEmpty ? nil : newValue)
                                }
                            }
                    }

                    // API key
                    HStack {
                        SecureField("API Key", text: $draftKey)
                            .textFieldStyle(.roundedBorder)
                        Button("Save") { onSave() }
                            .buttonStyle(.bordered)
                        if hasKey {
                            Button("Test") { onTest() }
                                .buttonStyle(.bordered)
                                .tint(.green)
                            Button("Delete", role: .destructive) { onDelete(); draftKey = "" }
                                .buttonStyle(.plain)
                                .foregroundColor(.red)
                        }
                    }
                    Button("Get API key →") {
                        NSWorkspace.shared.open(URL(string: provider.docURL)!)
                    }
                    .font(.caption)
                    .buttonStyle(.plain)
                    .foregroundColor(SF.Colors.purple)
                }
                .padding(12)
            }
        }
        .background(isSelected ? SF.Colors.purple.opacity(0.08) : SF.Colors.bgTertiary.opacity(0.5))
        .cornerRadius(10)
        .overlay(
            RoundedRectangle(cornerRadius: 10)
                .stroke(isSelected ? SF.Colors.purple.opacity(0.5) : Color.clear, lineWidth: 1.5)
        )
    }
}
