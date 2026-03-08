import SwiftUI

struct ContentView: View {
    @EnvironmentObject var keychain: KeychainService
    @StateObject private var appState = AppState.shared

    var body: some View {
        MainView()
            .environmentObject(appState)
            .frame(minWidth: 900, minHeight: 600)
    }
}
