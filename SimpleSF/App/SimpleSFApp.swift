import SwiftUI

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
                .preferredColorScheme(.dark)
                .background(SF.Colors.bgPrimary)
                .task {
                    await KeychainService.shared.scanIfNeeded()
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
