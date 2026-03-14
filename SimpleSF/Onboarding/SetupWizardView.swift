import SwiftUI

// Ref: FT-SSF-007
@MainActor
struct SetupWizardView: View {
    @ObservedObject var appState = AppState.shared
    @ObservedObject var hf = HuggingFaceService.shared
    @ObservedObject var mlx = MLXService.shared
    @ObservedObject var ollama = OllamaService.shared

    @State var step: SetupStep = .welcome
    @State var selectedModel: HuggingFaceService.HFModel?
    @State private var showAdvanced = false

    enum SetupStep {
        case welcome
        case chooseEngine
        case mlxModelPick
        case downloading
        case ollamaSetup
        case done
    }

    let ramGB = HuggingFaceService.systemRAMGB()
    let recommended = HuggingFaceService.recommendedModel()

    var body: some View {
        VStack(spacing: 0) {
            IHMContextHeader(context: .setupWizard)

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

    // MARK: - Helpers

    func chipName() -> String {
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
