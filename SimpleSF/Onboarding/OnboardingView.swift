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

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Image(systemName: "gearshape.fill").foregroundColor(.purple)
                Text("Settings")
                    .font(.title2.bold())
                Spacer()
                activeBadge
            }
            .padding()

            Divider()

            ScrollView {
                VStack(spacing: 20) {
                    // ── Language ──
                    languageSection

                    // ── Local LLM ──
                    localLLMSection

                    // ── Cloud Providers ──
                    cloudSection
                }
                .padding()
            }

            Divider()

            HStack(spacing: 16) {
                Image(systemName: "lock.shield.fill")
                    .foregroundColor(.green)
                VStack(alignment: .leading, spacing: 2) {
                    Text("Local models run 100% on your Mac. No data leaves your machine.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text("Cloud API keys stored in macOS Keychain — sent only to the provider.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .padding()
        }
        .onAppear {
            for p in LLMProvider.cloudProviders {
                if let k = keychain.key(for: p) { keys[p] = k }
            }
            mlx.scanModels()
            Task { await ollama.refresh() }
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
                .foregroundColor(.purple)
            Text("Language")
                .font(.headline)
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
                .foregroundColor(.secondary)
        }
        .padding()
        .background(Color(.controlBackgroundColor))
        .cornerRadius(12)
    }

    // MARK: - Active provider badge

    @ViewBuilder
    private var activeBadge: some View {
        if let prov = llm.activeProvider {
            HStack(spacing: 6) {
                Circle().fill(Color.green).frame(width: 8, height: 8)
                Text(prov.displayName)
                    .font(.caption.bold())
                if prov == .ollama, let m = ollama.activeModel {
                    Text("(\(m.name))")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                } else if prov == .mlx, let m = mlx.activeModel {
                    Text("(\(m.name))")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
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
                    .foregroundColor(.purple)
                Text("Local LLM")
                    .font(.headline)
                Spacer()
                Text("Zero network — runs on Apple Silicon")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }

            // Ollama card
            ollamaCard

            // MLX card
            mlxCard
        }
        .padding()
        .background(Color(.controlBackgroundColor))
        .cornerRadius(12)
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(
                    (ollama.isRunning || mlx.isRunning) ? Color.green.opacity(0.4) : Color.purple.opacity(0.15),
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
                if ollama.isRunning {
                    Label("Running", systemImage: "circle.fill")
                        .font(.caption2)
                        .foregroundColor(.green)
                } else {
                    Label("Stopped", systemImage: "circle")
                        .font(.caption2)
                        .foregroundColor(.gray)
                }
                Spacer()
                Button(action: { Task { await ollama.refresh() } }) {
                    Image(systemName: "arrow.clockwise")
                        .font(.caption)
                }
                .buttonStyle(.plain)
                .foregroundColor(.purple)
            }

            if ollama.isRunning {
                if !ollama.availableModels.isEmpty {
                    HStack {
                        Text("Model:")
                            .font(.callout)
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

                    if llm.activeProvider == .ollama {
                        HStack(spacing: 4) {
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundColor(.green)
                            Text("Active — Jarvis and agents use this model")
                                .font(.caption)
                                .foregroundColor(.green)
                        }
                    }
                } else {
                    Text("No models installed. Run: ollama pull qwen3:14b")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            } else {
                HStack(spacing: 12) {
                    Button(action: { ollama.start() }) {
                        Label("Start Ollama", systemImage: "play.circle.fill")
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(.purple)
                    .controlSize(.small)

                    Text("or run: ollama serve")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding(12)
        .background(Color(.textBackgroundColor).opacity(0.3))
        .cornerRadius(8)
    }

    // MARK: - MLX Card

    private var mlxCard: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("MLX")
                    .font(.subheadline.bold())
                mlxStatusBadge
                Spacer()
                Button(action: { mlx.scanModels() }) {
                    Image(systemName: "arrow.clockwise")
                        .font(.caption)
                }
                .buttonStyle(.plain)
                .foregroundColor(.purple)
            }

            if !mlx.availableModels.isEmpty {
                HStack {
                    Text("Model:")
                        .font(.callout)
                    Picker("", selection: $mlx.activeModel) {
                        ForEach(mlx.availableModels) { model in
                            Text(model.name).tag(model as MLXService.MLXModel?)
                        }
                    }
                    .labelsHidden()
                    .frame(maxWidth: 300)
                }
            } else {
                Text("No MLX models in ~/.cache/huggingface/hub/ or ~/.cache/mlx-models/")
                    .font(.caption)
                    .foregroundColor(.secondary)
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
                        .foregroundColor(.secondary)

                    if llm.activeProvider == .mlx {
                        HStack(spacing: 4) {
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundColor(.green)
                            Text("Active")
                                .font(.caption)
                                .foregroundColor(.green)
                        }
                    }
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
                    .tint(.purple)
                    .controlSize(.small)
                    .disabled(mlx.activeModel == nil)
                }
            }

            // Log output
            if !mlx.logLines.isEmpty {
                ScrollView(.vertical) {
                    VStack(alignment: .leading, spacing: 2) {
                        ForEach(Array(mlx.logLines.suffix(3).enumerated()), id: \.offset) { _, line in
                            Text(line)
                                .font(.caption2.monospaced())
                                .foregroundColor(.secondary)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                .frame(maxHeight: 40)
            }
        }
        .padding(12)
        .background(Color(.textBackgroundColor).opacity(0.3))
        .cornerRadius(8)
    }

    @ViewBuilder
    private var mlxStatusBadge: some View {
        switch mlx.state {
        case .stopped:
            Label("Stopped", systemImage: "circle")
                .font(.caption2)
                .foregroundColor(.gray)
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
                Spacer()
                Text("Requires API key")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }

            ForEach(LLMProvider.cloudProviders, id: \.self) { provider in
                ProviderRow(
                    provider: provider,
                    storedKey: keychain.key(for: provider),
                    draftKey: Binding(
                        get: { keys[provider] ?? "" },
                        set: { keys[provider] = $0 }
                    ),
                    isTesting: testing == provider,
                    testResult: testResults[provider],
                    onSave: { save(provider: provider) },
                    onTest: { Task { await test(provider: provider) } },
                    onDelete: { keychain.delete(for: provider) }
                )
            }
        }
        .padding()
        .background(Color(.controlBackgroundColor))
        .cornerRadius(12)
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

struct ProviderRow: View {
    let provider: LLMProvider
    let storedKey: String?
    @Binding var draftKey: String
    let isTesting: Bool
    let testResult: Bool?
    let onSave: () -> Void
    let onTest: () -> Void
    let onDelete: () -> Void

    @State private var expanded = false

    private var hasKey: Bool { storedKey != nil && !storedKey!.isEmpty }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Row header
            Button(action: { withAnimation(.spring(response: 0.3)) { expanded.toggle() } }) {
                HStack {
                    Circle()
                        .fill(hasKey ? Color.green : Color.gray.opacity(0.3))
                        .frame(width: 8, height: 8)
                    Text(provider.displayName)
                        .font(.headline)
                    Text("· \(provider.defaultModel)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Spacer()
                    if hasKey {
                        if isTesting {
                            ProgressView().scaleEffect(0.7)
                        } else if let ok = testResult {
                            Image(systemName: ok ? "checkmark.circle.fill" : "xmark.circle.fill")
                                .foregroundColor(ok ? .green : .red)
                        }
                    }
                    Image(systemName: expanded ? "chevron.up" : "chevron.down")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding(12)
            }
            .buttonStyle(.plain)

            if expanded {
                Divider()
                VStack(alignment: .leading, spacing: 10) {
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
                    .foregroundColor(.purple)
                }
                .padding(12)
            }
        }
        .background(Color(.controlBackgroundColor))
        .cornerRadius(10)
    }
}
