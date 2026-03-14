import SwiftUI

// Ref: FT-SSF-007, FT-SSF-015
// Individual wizard step views: Welcome, Choose Engine, Ollama Setup, Done.

extension SetupWizardView {

    // MARK: - Step 1: Welcome

    var welcomeStep: some View {
        VStack(spacing: 24) {
            Image(systemName: "sparkles")
                .font(.system(size: 64))
                .foregroundStyle(
                    LinearGradient(colors: [.purple, .blue], startPoint: .topLeading, endPoint: .bottomTrailing)
                )

            Text(L10n.shared.t(.setupTitle))
                .font(.largeTitle.bold())
                .foregroundColor(.white)

            Text(L10n.shared.t(.setupSubtitle))
                .font(.title3)
                .foregroundColor(.gray)
                .multilineTextAlignment(.center)
                .lineSpacing(4)

            VStack(spacing: 8) {
                HStack(spacing: 8) {
                    Image(systemName: "cpu")
                        .foregroundColor(.purple)
                    Text("Apple \(chipName()) · \(ramGB) GB unified memory")
                        .font(.callout.monospaced())
                        .foregroundColor(.white.opacity(0.8))
                }
                .padding(.horizontal, 16)
                .padding(.vertical, 8)
                .background(Color.white.opacity(0.06))
                .cornerRadius(8)
            }

            Button(action: { withAnimation { step = .chooseEngine } }) {
                Label(L10n.shared.t(.actionGetStarted), systemImage: "arrow.right.circle.fill")
                    .font(.title3.bold())
                    .padding(.horizontal, 32)
                    .padding(.vertical, 12)
            }
            .buttonStyle(.borderedProminent)
            .tint(.purple)
            .padding(.top, 8)

            Button(L10n.shared.t(.setupSkipConfigure)) {
                appState.completeSetup()
            }
            .font(.caption)
            .foregroundColor(.gray)
        }
        .padding(40)
    }

    // MARK: - Step 2: Choose Engine

    var chooseEngineStep: some View {
        VStack(spacing: 24) {
            Text(L10n.shared.t(.setupHowRun))
                .font(.title2.bold())
                .foregroundColor(.white)

            Text(L10n.shared.t(.setupPrivacy))
                .font(.callout)
                .foregroundColor(.gray)

            HStack(spacing: 20) {
                engineCard(
                    icon: "apple.logo",
                    title: "MLX (Recommended)",
                    subtitle: "Apple Silicon optimized\nFastest on M-series chips",
                    recommended: true
                ) {
                    selectedModel = recommended
                    appState.setPreferredProvider("mlx")
                    withAnimation { step = .mlxModelPick }
                }

                engineCard(
                    icon: "terminal",
                    title: "Ollama",
                    subtitle: "Universal engine\nSupports thousands of models",
                    recommended: false
                ) {
                    appState.setPreferredProvider("ollama")
                    withAnimation { step = .ollamaSetup }
                }
            }
            .frame(maxWidth: 600)

            Button("Skip — use cloud API keys instead") {
                appState.completeSetup()
            }
            .font(.caption)
            .foregroundColor(.gray)
            .padding(.top, 8)

            Button(action: { withAnimation { step = .welcome } }) {
                Label(L10n.shared.t(.actionBack), systemImage: "arrow.left")
                    .font(.caption)
            }
            .buttonStyle(.plain)
            .foregroundColor(.gray)
        }
        .padding(40)
    }

    func engineCard(icon: String, title: String, subtitle: String, recommended: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            VStack(spacing: 16) {
                Image(systemName: icon)
                    .font(.system(size: 40))
                    .foregroundColor(recommended ? .purple : .blue)

                Text(title)
                    .font(.headline)
                    .foregroundColor(.white)

                Text(subtitle)
                    .font(.caption)
                    .foregroundColor(.gray)
                    .multilineTextAlignment(.center)
                    .lineSpacing(2)
            }
            .frame(maxWidth: .infinity)
            .padding(24)
            .background(Color.white.opacity(recommended ? 0.08 : 0.04))
            .cornerRadius(16)
            .overlay(
                RoundedRectangle(cornerRadius: 16)
                    .stroke(recommended ? Color.purple.opacity(0.5) : Color.gray.opacity(0.2), lineWidth: 1.5)
            )
        }
        .buttonStyle(.plain)
    }

    // MARK: - Step 3c: Ollama Setup

    var ollamaStep: some View {
        VStack(spacing: 24) {
            Image(systemName: "terminal")
                .font(.system(size: 48))
                .foregroundColor(.blue)

            Text(L10n.shared.t(.setupOllama))
                .font(.title2.bold())
                .foregroundColor(.white)

            if ollama.isRunning {
                Label(L10n.shared.t(.setupOllamaRunning), systemImage: "checkmark.circle.fill")
                    .font(.headline)
                    .foregroundColor(.green)

                if !ollama.availableModels.isEmpty {
                    VStack(alignment: .leading, spacing: 8) {
                        Text(L10n.shared.t(.setupOllamaAvailable))
                            .font(.callout)
                            .foregroundColor(.gray)

                        ForEach(ollama.availableModels) { model in
                            HStack {
                                Image(systemName: "cube.fill")
                                    .foregroundColor(.blue)
                                Text(model.name)
                                    .font(.callout.bold())
                                    .foregroundColor(.white)
                                Text(model.size)
                                    .font(.caption)
                                    .foregroundColor(.gray)
                            }
                            .padding(8)
                            .frame(maxWidth: 400, alignment: .leading)
                            .background(Color.white.opacity(0.04))
                            .cornerRadius(8)
                        }
                    }

                    Button(action: { withAnimation { step = .done } }) {
                        Label("Continue with Ollama", systemImage: "arrow.right.circle.fill")
                            .font(.headline)
                            .padding(.horizontal, 32)
                            .padding(.vertical, 12)
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(.blue)
                } else {
                    Text(L10n.shared.t(.setupOllamaNoModels))
                        .font(.callout)
                        .foregroundColor(.gray)

                    Text("ollama pull qwen3:14b")
                        .font(.callout.monospaced())
                        .padding(8)
                        .background(Color.black.opacity(0.3))
                        .cornerRadius(6)
                        .foregroundColor(.green)
                        .textSelection(.enabled)

                    Button("Refresh") {
                        Task { await ollama.refresh() }
                    }
                    .buttonStyle(.bordered)
                    .tint(.blue)
                }
            } else {
                VStack(spacing: 16) {
                    Text(L10n.shared.t(.setupOllamaNotRunning))
                        .font(.callout)
                        .foregroundColor(.gray)

                    HStack(spacing: 16) {
                        Button(action: { ollama.start() }) {
                            Label(L10n.shared.t(.setupOllamaOpen), systemImage: "play.circle.fill")
                                .padding(.horizontal, 16)
                                .padding(.vertical, 8)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.blue)

                        Link(destination: URL(string: "https://ollama.com/download")!) {
                            Label(L10n.shared.t(.setupOllamaInstall), systemImage: "arrow.down.circle")
                                .padding(.horizontal, 16)
                                .padding(.vertical, 8)
                        }
                        .buttonStyle(.bordered)
                    }

                    Button("Refresh") {
                        Task { await ollama.refresh() }
                    }
                    .font(.caption)
                    .foregroundColor(.gray)
                }
            }

            HStack {
                Button(action: { withAnimation { step = .chooseEngine } }) {
                    Label(L10n.shared.t(.actionBack), systemImage: "arrow.left")
                }
                .buttonStyle(.plain)
                .foregroundColor(.gray)
                Spacer()
            }
            .frame(maxWidth: 400)
        }
        .padding(40)
        .onAppear { Task { await ollama.refresh() } }
    }

    // MARK: - Step 4: Done

    var doneStep: some View {
        VStack(spacing: 24) {
            Image(systemName: "checkmark.seal.fill")
                .font(.system(size: 64))
                .foregroundStyle(
                    LinearGradient(colors: [.green, .purple], startPoint: .topLeading, endPoint: .bottomTrailing)
                )

            Text(L10n.shared.t(.setupAllSet))
                .font(.largeTitle.bold())
                .foregroundColor(.white)

            Text(L10n.shared.t(.setupAllSetSubtitle))
                .font(.title3)
                .foregroundColor(.gray)
                .multilineTextAlignment(.center)

            VStack(alignment: .leading, spacing: 8) {
                if let model = selectedModel, hf.isDownloaded(model) {
                    HStack(spacing: 8) {
                        Image(systemName: "apple.logo")
                            .foregroundColor(.purple)
                        Text("MLX · \(model.name)")
                            .foregroundColor(.white)
                    }
                } else if ollama.isRunning {
                    HStack(spacing: 8) {
                        Image(systemName: "terminal")
                            .foregroundColor(.blue)
                        Text("Ollama · \(ollama.activeModel?.name ?? "ready")")
                            .foregroundColor(.white)
                    }
                }
            }
            .font(.callout)
            .padding(12)
            .background(Color.white.opacity(0.06))
            .cornerRadius(8)

            Button(action: { appState.completeSetup() }) {
                Label(L10n.shared.t(.setupStartUsing), systemImage: "sparkles")
                    .font(.title3.bold())
                    .padding(.horizontal, 32)
                    .padding(.vertical, 12)
            }
            .buttonStyle(.borderedProminent)
            .tint(.purple)
            .padding(.top, 8)
        }
        .padding(40)
    }
}
