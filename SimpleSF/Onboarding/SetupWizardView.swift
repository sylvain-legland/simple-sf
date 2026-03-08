import SwiftUI

@MainActor
struct SetupWizardView: View {
    @ObservedObject private var appState = AppState.shared
    @ObservedObject private var hf = HuggingFaceService.shared
    @ObservedObject private var mlx = MLXService.shared
    @ObservedObject private var ollama = OllamaService.shared

    @State private var step: SetupStep = .welcome
    @State private var selectedModel: HuggingFaceService.HFModel?
    @State private var showAdvanced = false

    enum SetupStep {
        case welcome
        case chooseEngine
        case mlxModelPick
        case downloading
        case ollamaSetup
        case done
    }

    private let ramGB = HuggingFaceService.systemRAMGB()
    private let recommended = HuggingFaceService.recommendedModel()

    var body: some View {
        VStack(spacing: 0) {
            // Progress dots
            progressDots
                .padding(.top, 24)

            Spacer()

            Group {
                switch step {
                case .welcome:      welcomeStep
                case .chooseEngine: chooseEngineStep
                case .mlxModelPick: mlxModelStep
                case .downloading:  downloadingStep
                case .ollamaSetup:  ollamaStep
                case .done:         doneStep
                }
            }
            .transition(.opacity.combined(with: .move(edge: .trailing)))

            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(
            LinearGradient(
                colors: [Color(red: 0.06, green: 0.04, blue: 0.10),
                         Color(red: 0.10, green: 0.05, blue: 0.18)],
                startPoint: .top, endPoint: .bottom
            )
        )
        .animation(.easeInOut(duration: 0.3), value: step)
    }

    // MARK: - Progress

    private var progressDots: some View {
        HStack(spacing: 8) {
            ForEach(0..<4) { i in
                Circle()
                    .fill(dotIndex >= i ? Color.purple : Color.gray.opacity(0.3))
                    .frame(width: 8, height: 8)
            }
        }
    }

    private var dotIndex: Int {
        switch step {
        case .welcome: return 0
        case .chooseEngine: return 1
        case .mlxModelPick, .ollamaSetup: return 2
        case .downloading: return 2
        case .done: return 3
        }
    }

    // MARK: - Step 1: Welcome

    private var welcomeStep: some View {
        VStack(spacing: 24) {
            Image(systemName: "sparkles")
                .font(.system(size: 64))
                .foregroundStyle(
                    LinearGradient(colors: [.purple, .blue], startPoint: .topLeading, endPoint: .bottomTrailing)
                )

            Text("Simple SF")
                .font(.largeTitle.bold())
                .foregroundColor(.white)

            Text("Your private Software Factory\nPowered by AI agents, running 100% on your Mac.")
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
                Label("Get Started", systemImage: "arrow.right.circle.fill")
                    .font(.title3.bold())
                    .padding(.horizontal, 32)
                    .padding(.vertical, 12)
            }
            .buttonStyle(.borderedProminent)
            .tint(.purple)
            .padding(.top, 8)

            Button("Skip — I'll configure later") {
                appState.completeSetup()
            }
            .font(.caption)
            .foregroundColor(.gray)
        }
        .padding(40)
    }

    // MARK: - Step 2: Choose Engine

    private var chooseEngineStep: some View {
        VStack(spacing: 24) {
            Text("How do you want to run AI?")
                .font(.title2.bold())
                .foregroundColor(.white)

            Text("Everything stays on your Mac. No data leaves your machine.")
                .font(.callout)
                .foregroundColor(.gray)

            HStack(spacing: 20) {
                // MLX card
                engineCard(
                    icon: "apple.logo",
                    title: "MLX (Recommended)",
                    subtitle: "Apple Silicon optimized\nFastest on M-series chips",
                    recommended: true
                ) {
                    selectedModel = recommended
                    withAnimation { step = .mlxModelPick }
                }

                // Ollama card
                engineCard(
                    icon: "terminal",
                    title: "Ollama",
                    subtitle: "Universal engine\nSupports thousands of models",
                    recommended: false
                ) {
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
                Label("Back", systemImage: "arrow.left")
                    .font(.caption)
            }
            .buttonStyle(.plain)
            .foregroundColor(.gray)
        }
        .padding(40)
    }

    private func engineCard(icon: String, title: String, subtitle: String, recommended: Bool, action: @escaping () -> Void) -> some View {
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

    // MARK: - Step 3a: MLX Model Pick

    private var mlxModelStep: some View {
        VStack(spacing: 20) {
            Text("Choose your model")
                .font(.title2.bold())
                .foregroundColor(.white)

            HStack(spacing: 6) {
                Image(systemName: "memorychip")
                    .foregroundColor(.purple)
                Text("\(ramGB) GB unified memory — recommended:")
                    .font(.callout)
                    .foregroundColor(.gray)
                Text(recommended.name)
                    .font(.callout.bold())
                    .foregroundColor(.purple)
            }

            VStack(spacing: 8) {
                ForEach(HuggingFaceService.curatedModels) { model in
                    let fits = model.minRAMGB <= ramGB
                    let isRec = model.repoId == recommended.repoId
                    let isSelected = selectedModel?.repoId == model.repoId
                    let alreadyHave = hf.isDownloaded(model)

                    Button(action: { if fits { selectedModel = model } }) {
                        HStack(spacing: 12) {
                            Image(systemName: isSelected ? "checkmark.circle.fill" : "circle")
                                .foregroundColor(isSelected ? .purple : .gray)

                            VStack(alignment: .leading, spacing: 2) {
                                HStack {
                                    Text(model.name)
                                        .font(.headline)
                                        .foregroundColor(fits ? .white : .gray)
                                    if isRec {
                                        Text("Recommended")
                                            .font(.caption2.bold())
                                            .padding(.horizontal, 6)
                                            .padding(.vertical, 2)
                                            .background(Color.purple)
                                            .foregroundColor(.white)
                                            .cornerRadius(4)
                                    }
                                    if alreadyHave {
                                        Text("Installed")
                                            .font(.caption2.bold())
                                            .padding(.horizontal, 6)
                                            .padding(.vertical, 2)
                                            .background(Color.green.opacity(0.2))
                                            .foregroundColor(.green)
                                            .cornerRadius(4)
                                    }
                                }
                                Text("\(model.params) · \(model.quant) · \(String(format: "%.1f", model.sizeGB)) GB · min \(model.minRAMGB) GB RAM")
                                    .font(.caption)
                                    .foregroundColor(fits ? .gray : .red.opacity(0.7))
                            }

                            Spacer()

                            if !fits {
                                Image(systemName: "exclamationmark.triangle.fill")
                                    .foregroundColor(.red.opacity(0.5))
                            }
                        }
                        .padding(12)
                        .background(isSelected ? Color.purple.opacity(0.12) : Color.white.opacity(0.04))
                        .cornerRadius(10)
                        .overlay(
                            RoundedRectangle(cornerRadius: 10)
                                .stroke(isSelected ? Color.purple.opacity(0.4) : Color.clear, lineWidth: 1)
                        )
                    }
                    .buttonStyle(.plain)
                    .disabled(!fits)
                }
            }
            .frame(maxWidth: 550)

            HStack(spacing: 16) {
                Button(action: { withAnimation { step = .chooseEngine } }) {
                    Label("Back", systemImage: "arrow.left")
                }
                .buttonStyle(.plain)
                .foregroundColor(.gray)

                Spacer()

                if let model = selectedModel {
                    if hf.isDownloaded(model) {
                        Button(action: {
                            mlx.scanModels()
                            withAnimation { step = .done }
                        }) {
                            Label("Use this model", systemImage: "checkmark.circle.fill")
                                .font(.headline)
                                .padding(.horizontal, 24)
                                .padding(.vertical, 10)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.green)
                    } else {
                        Button(action: {
                            hf.download(model: model)
                            withAnimation { step = .downloading }
                        }) {
                            Label("Download \(String(format: "%.1f", model.sizeGB)) GB", systemImage: "arrow.down.circle.fill")
                                .font(.headline)
                                .padding(.horizontal, 24)
                                .padding(.vertical, 10)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.purple)
                    }
                }
            }
            .frame(maxWidth: 550)
        }
        .padding(40)
    }

    // MARK: - Step 3b: Downloading

    private var downloadingStep: some View {
        VStack(spacing: 24) {
            if case .completed = hf.downloadState {
                Image(systemName: "checkmark.circle.fill")
                    .font(.system(size: 56))
                    .foregroundColor(.green)

                Text("Download complete!")
                    .font(.title2.bold())
                    .foregroundColor(.white)

                Text(selectedModel?.name ?? "Model")
                    .font(.title3)
                    .foregroundColor(.purple)

                Button(action: {
                    mlx.scanModels()
                    withAnimation { step = .done }
                }) {
                    Label("Continue", systemImage: "arrow.right.circle.fill")
                        .font(.headline)
                        .padding(.horizontal, 32)
                        .padding(.vertical, 12)
                }
                .buttonStyle(.borderedProminent)
                .tint(.purple)
            } else if case .failed(let msg) = hf.downloadState {
                Image(systemName: "xmark.circle.fill")
                    .font(.system(size: 56))
                    .foregroundColor(.red)

                Text("Download failed")
                    .font(.title2.bold())
                    .foregroundColor(.white)

                Text(msg)
                    .font(.caption)
                    .foregroundColor(.red)

                Button("Retry") {
                    if let model = selectedModel {
                        hf.downloadState = .idle
                        hf.download(model: model)
                    }
                }
                .buttonStyle(.borderedProminent)
                .tint(.purple)

                Button("Back") {
                    hf.downloadState = .idle
                    withAnimation { step = .mlxModelPick }
                }
                .foregroundColor(.gray)
            } else {
                ProgressView()
                    .scaleEffect(1.5)
                    .padding(.bottom, 8)

                Text("Downloading \(selectedModel?.name ?? "model")...")
                    .font(.title3.bold())
                    .foregroundColor(.white)

                Text("\(String(format: "%.1f", selectedModel?.sizeGB ?? 0)) GB from HuggingFace")
                    .font(.callout)
                    .foregroundColor(.gray)

                // Progress log
                ScrollView {
                    VStack(alignment: .leading, spacing: 2) {
                        ForEach(Array(hf.downloadLog.suffix(8).enumerated()), id: \.offset) { _, line in
                            Text(line)
                                .font(.caption2.monospaced())
                                .foregroundColor(.gray)
                                .lineLimit(1)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                .frame(maxWidth: 500, maxHeight: 120)
                .padding(8)
                .background(Color.black.opacity(0.3))
                .cornerRadius(8)

                Button("Cancel") {
                    hf.cancelDownload()
                    hf.downloadState = .idle
                    withAnimation { step = .mlxModelPick }
                }
                .font(.caption)
                .foregroundColor(.red.opacity(0.7))
            }
        }
        .padding(40)
    }

    // MARK: - Step 3c: Ollama Setup

    private var ollamaStep: some View {
        VStack(spacing: 24) {
            Image(systemName: "terminal")
                .font(.system(size: 48))
                .foregroundColor(.blue)

            Text("Ollama Setup")
                .font(.title2.bold())
                .foregroundColor(.white)

            if ollama.isRunning {
                Label("Ollama is running", systemImage: "checkmark.circle.fill")
                    .font(.headline)
                    .foregroundColor(.green)

                if !ollama.availableModels.isEmpty {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Available models:")
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
                    Text("No models installed. Run in Terminal:")
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
                    Text("Ollama is not running")
                        .font(.callout)
                        .foregroundColor(.gray)

                    HStack(spacing: 16) {
                        Button(action: { ollama.start() }) {
                            Label("Open Ollama.app", systemImage: "play.circle.fill")
                                .padding(.horizontal, 16)
                                .padding(.vertical, 8)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.blue)

                        Link(destination: URL(string: "https://ollama.com/download")!) {
                            Label("Install Ollama", systemImage: "arrow.down.circle")
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
                    Label("Back", systemImage: "arrow.left")
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

    private var doneStep: some View {
        VStack(spacing: 24) {
            Image(systemName: "checkmark.seal.fill")
                .font(.system(size: 64))
                .foregroundStyle(
                    LinearGradient(colors: [.green, .purple], startPoint: .topLeading, endPoint: .bottomTrailing)
                )

            Text("You're all set!")
                .font(.largeTitle.bold())
                .foregroundColor(.white)

            Text("Your private AI Software Factory is ready.\nTalk to Jarvis to start building.")
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
                Label("Start using Simple SF", systemImage: "sparkles")
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

    // MARK: - Helpers

    private func chipName() -> String {
        var size = 0
        sysctlbyname("machdep.cpu.brand_string", nil, &size, nil, 0)
        var result = [CChar](repeating: 0, count: size)
        sysctlbyname("machdep.cpu.brand_string", &result, &size, nil, 0)
        let full = String(cString: result)
        if full.contains("Apple") { return full }
        // Fallback for Apple Silicon
        return "M-series"
    }
}
