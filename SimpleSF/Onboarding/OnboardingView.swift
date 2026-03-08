import SwiftUI

@MainActor
struct OnboardingView: View {
    @ObservedObject private var keychain = KeychainService.shared
    @ObservedObject private var llm = LLMService.shared
    @State private var keys: [LLMProvider: String] = [:]
    @State private var testing: LLMProvider? = nil
    @State private var testResults: [LLMProvider: Bool] = [:]
    @State private var selectedProvider: LLMProvider = .minimax

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Image(systemName: "key.fill").foregroundColor(.yellow)
                Text("API Keys")
                    .font(.title2.bold())
                Spacer()
                if let prov = llm.activeProvider {
                    Label("Active: \(prov.displayName)", systemImage: "checkmark.circle.fill")
                        .font(.caption)
                        .foregroundColor(.green)
                }
            }
            .padding()

            Divider()

            ScrollView {
                VStack(spacing: 8) {
                    ForEach(LLMProvider.allCases, id: \.self) { provider in
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
            }

            Divider()

            // Footer info
            VStack(spacing: 4) {
                Text("Keys stored in macOS Keychain — never sent anywhere except directly to the provider.")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Text("All LLM calls go directly from this app to the provider. No relay server.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .padding()
        }
        .onAppear {
            // Pre-fill text fields with existing keys
            for p in LLMProvider.allCases {
                if let k = keychain.key(for: p) { keys[p] = k }
            }
        }
    }

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
