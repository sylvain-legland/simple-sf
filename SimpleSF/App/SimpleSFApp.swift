import SwiftUI

@main
struct SimpleSFApp: App {
    @StateObject private var keychain = KeychainService.shared

    init() {
        NSApplication.shared.setActivationPolicy(.regular)
        NSApplication.shared.activate(ignoringOtherApps: true)
        // Initialize the Rust SF engine
        SFBridge.shared.initialize()
        // Pass LLM config from Keychain to Rust engine
        SFBridge.shared.syncLLMConfig()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(keychain)
                .frame(minWidth: 1000, minHeight: 650)
                .preferredColorScheme(.dark)
                .background(SF.Colors.bgPrimary)
        }
        .defaultSize(width: 1200, height: 800)
        .windowResizability(.contentMinSize)
        .commands {
            CommandGroup(replacing: .newItem) {}
        }
    }
}
