import SwiftUI

struct OnboardingView: View {
    @EnvironmentObject var appState: AppState
    @State private var keys: [LLMProvider: String] = [:]
    @State private var testResults: [LLMProvider: TestResult] = [:]
    @State private var step: Int = 0

    enum TestResult { case none, testing, ok, fail(String) }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            VStack(spacing: 8) {
                Image(systemName: "key.fill")
                    .resizable().scaledToFit().frame(width: 40)
                    .foregroundStyle(.purple)
                Text("Configure your LLM providers")
                    .font(.title2.bold())
                Text("Add at least one API key to get started. Keys are stored securely in your macOS Keychain.")
                    .font(.callout).foregroundStyle(.secondary)
                    .multilineTextAlignment(.center).frame(maxWidth: 480)
            }
            .padding(.vertical, 32)

            Divider()

            // Provider list
            ScrollView {
                VStack(spacing: 12) {
                    ForEach(LLMProvider.allCases) { provider in
                        ProviderRow(
                            provider: provider,
                            key: Binding(
                                get: { keys[provider] ?? KeychainStore.shared.getKey(for: provider) ?? "" },
                                set: { keys[provider] = $0 }
                            ),
                            testResult: testResults[provider] ?? .none,
                            onSave: { saveKey(provider) },
                            onTest: { await testKey(provider) }
                        )
                    }
                }
                .padding(.horizontal, 40)
                .padding(.vertical, 20)
            }

            Divider()

            // Footer
            HStack {
                Spacer()
                Button("Skip for now") { appState.completeOnboarding() }
                    .buttonStyle(.plain).foregroundStyle(.secondary)
                Button("Continue →") { appState.completeOnboarding() }
                    .buttonStyle(.borderedProminent)
                    .disabled(!KeychainStore.shared.hasAnyKey())
            }
            .padding(.horizontal, 40)
            .padding(.vertical, 20)
        }
        .frame(width: 680, height: 640)
    }

    private func saveKey(_ provider: LLMProvider) {
        guard let key = keys[provider], !key.isEmpty else {
            KeychainStore.shared.deleteKey(for: provider)
            return
        }
        KeychainStore.shared.setKey(key, for: provider)
    }

    private func testKey(_ provider: LLMProvider) async {
        guard let key = keys[provider], !key.isEmpty else { return }
        testResults[provider] = .testing
        do {
            try await LLMConnectionTest.test(provider: provider, key: key)
            testResults[provider] = .ok
        } catch {
            testResults[provider] = .fail(error.localizedDescription)
        }
    }
}

struct ProviderRow: View {
    let provider: LLMProvider
    @Binding var key: String
    let testResult: OnboardingView.TestResult
    let onSave: () -> Void
    let onTest: () async -> Void

    @State private var isEditing = false
    @State private var isTesting = false

    var body: some View {
        HStack(spacing: 12) {
            // Icon + name
            VStack(alignment: .leading, spacing: 2) {
                Text(provider.displayName).font(.headline)
                if let url = provider.docURL {
                    Link("Get API key", destination: url)
                        .font(.caption).foregroundStyle(.purple)
                }
            }
            .frame(width: 160, alignment: .leading)

            // Key input
            SecureField("API key", text: $key)
                .textFieldStyle(.roundedBorder)
                .onChange(of: key) { _, _ in onSave() }

            // Status indicator
            statusView

            // Test button
            Button {
                isTesting = true
                Task { await onTest(); isTesting = false }
            } label: {
                Label("Test", systemImage: "bolt.fill")
                    .font(.caption)
            }
            .disabled(key.isEmpty || isTesting)
            .buttonStyle(.bordered)
        }
        .padding(12)
        .background(RoundedRectangle(cornerRadius: 10).fill(.secondary.opacity(0.08)))
    }

    @ViewBuilder
    private var statusView: some View {
        switch testResult {
        case .none:
            Circle().fill(.clear).frame(width: 10)
        case .testing:
            ProgressView().scaleEffect(0.6).frame(width: 20)
        case .ok:
            Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
        case .fail(let msg):
            Image(systemName: "xmark.circle.fill").foregroundStyle(.red)
                .help(msg)
        }
    }
}
