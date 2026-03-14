import SwiftUI

// Ref: FT-SSF-001
@main
struct SimpleSFApp: App {

    init() {
        NSApplication.shared.setActivationPolicy(.regular)
        NSApplication.shared.activate(ignoringOtherApps: true)
        SFBridge.shared.initialize()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(KeychainService.shared)
                .frame(minWidth: 1000, minHeight: 650)
                .background(SF.Colors.bgPrimary)
                .task {
                    await KeychainService.shared.scanIfNeeded()
                    // Auto-restart MLX server if selected and enabled
                    let state = AppState.shared
                    if state.mlxAutoRestart,
                       (state.selectedProvider == .mlx || state.preferredLocalProvider == "mlx"),
                       !MLXService.shared.isRunning {
                        MLXService.shared.start()
                    }
                    await SFBridge.shared.syncLLMConfigAsync()
                }
        }
        .defaultSize(width: 1200, height: 800)
        .windowResizability(.contentMinSize)
        .commands {
            CommandGroup(replacing: .newItem) {}
        }
    }
}
