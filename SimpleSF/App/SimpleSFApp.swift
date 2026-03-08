import SwiftUI
import SwiftData

@main
struct SimpleSFApp: App {
    @StateObject private var launcher = PlatformLauncher.shared
    @StateObject private var appState = AppState.shared

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(launcher)
                .environmentObject(appState)
                .task { await launcher.start() }
        }
        .windowStyle(.automatic)
        .commands {
            CommandGroup(replacing: .appInfo) {
                Button("About Simple SF") { appState.showAbout = true }
            }
        }

        Settings {
            SettingsRootView()
                .environmentObject(appState)
        }
    }
}
