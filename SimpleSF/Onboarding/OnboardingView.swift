import SwiftUI

@MainActor
struct OnboardingView: View {
    @ObservedObject private var keychain = KeychainService.shared
    @ObservedObject private var llm = LLMService.shared
    @ObservedObject private var mlx = MLXService.shared
    @State private var keys: [LLMProvider: String] = [:]
    @State private var testing: LLMProvider? = nil
    @State private var testResults: [LLMProvider: Bool] = [:]

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Image(systemName: "key.fill").foregroundColor(.yellow)
                Text("LLM Configuration")
                    .font(.title2.bold())
                Spacer()
                activeBadge
            }
            .padding()

            Divider()

            ScrollView {
                VStack(spacing: 16) {
                    // MLX Local section
                    mlxSection

                    Divider().padding(.horizontal)

                    // Cloud providers
                    VStack(spacing: 8) {
                        HStack {
                            Image(systemName: "cloud.fill")
                                .foregroundColor(.blue)
                            Text("Cloud Providers")
                                .font(.headline)
                            Spacer()
                        }
                        .padding(.horizontal)

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
                }
                .padding()
            }

            Divider()

            VStack(spacing: 4) {
                Text("Keys stored in macOS Keychain — never sent anywhere except directly to the provider.")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Text("MLX runs 100% locally on your Mac. No data leaves your machine.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .padding()
        }
        .onAppear {
            for p in LLMProvider.cloudProviders {
                if let k = keychain.key(for: p) { keys[p] = k }
            }
            mlx.scanModels()
        }
    }

    // MARK: - Active provider badge

    @ViewBuilder
    private var activeBadge: some View {
        if mlx.isRunning {
            Label("MLX Local", systemImage: "cpu")
                .font(.caption)
                .foregroundColor(.green)
        } else if let prov = llm.activeProvider {
            Label(prov.displayName, systemImage: "checkmark.circle.fill")
                .font(.caption)
                .foregroundColor(.green)
        } else {
            Label("No provider", systemImage: "exclamationmark.circle")
                .font(.caption)
                .foregroundColor(.orange)
        }
    }

    // MARK: - MLX Section

    private var mlxSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Image(systemName: "cpu")
                    .foregroundColor(.purple)
                Text("MLX Local")
                    .font(.headline)
                Spacer()
                mlxStatusBadge
            }

            // Model picker
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
                }
            } else {
                Text("No MLX models found in ~/.cache/huggingface/hub/")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            // Start / Stop buttons
            HStack(spacing: 12) {
                if mlx.isRunning {
                    Button(action: {
                        mlx.stop()
                    }) {
                        Label("Stop Server", systemImage: "stop.circle.fill")
                    }
                    .buttonStyle(.bordered)
                    .tint(.red)

                    Text("Port \(mlx.port)")
                        .font(.caption.monospaced())
                        .foregroundColor(.secondary)
                } else {
                    Button(action: {
                        mlx.start()
                        // Sync to Rust engine after a delay
                        Task {
                            try? await Task.sleep(nanoseconds: 5_000_000_000)
                            if mlx.isRunning {
                                SFBridge.shared.syncLLMConfig()
                            }
                        }
                    }) {
                        Label("Start Server", systemImage: "play.circle.fill")
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(.purple)
                    .disabled(mlx.activeModel == nil)

                    Button(action: { mlx.scanModels() }) {
                        Image(systemName: "arrow.clockwise")
                    }
                    .buttonStyle(.bordered)
                }
            }

            // Log output
            if !mlx.logLines.isEmpty {
                ScrollView(.vertical) {
                    VStack(alignment: .leading, spacing: 2) {
                        ForEach(Array(mlx.logLines.suffix(5).enumerated()), id: \.offset) { _, line in
                            Text(line)
                                .font(.caption2.monospaced())
                                .foregroundColor(.secondary)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                .frame(maxHeight: 60)
            }
        }
        .padding()
        .background(Color(.controlBackgroundColor))
        .cornerRadius(10)
        .overlay(
            RoundedRectangle(cornerRadius: 10)
                .stroke(mlx.isRunning ? Color.green.opacity(0.4) : Color.purple.opacity(0.2), lineWidth: 1)
        )
    }

    @ViewBuilder
    private var mlxStatusBadge: some View {
        switch mlx.state {
        case .stopped:
            Label("Stopped", systemImage: "circle")
                .font(.caption)
                .foregroundColor(.gray)
        case .starting:
            HStack(spacing: 4) {
                ProgressView().scaleEffect(0.6)
                Text("Starting...")
                    .font(.caption)
                    .foregroundColor(.orange)
            }
        case .running:
            Label("Running", systemImage: "circle.fill")
                .font(.caption)
                .foregroundColor(.green)
        case .error(let msg):
            Label(msg, systemImage: "exclamationmark.circle.fill")
                .font(.caption)
                .foregroundColor(.red)
                .lineLimit(1)
        }
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
