import SwiftUI

struct ContentView: View {
    @EnvironmentObject var keychain: KeychainService
    @ObservedObject private var appState = AppState.shared

    var body: some View {
        if appState.hasCompletedSetup {
            MainView()
                .frame(minWidth: 1000, minHeight: 650)
        } else {
            SetupWizardView()
                .frame(minWidth: 800, minHeight: 600)
        }
    }
}
