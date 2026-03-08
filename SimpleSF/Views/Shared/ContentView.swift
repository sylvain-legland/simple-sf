import SwiftUI

struct ContentView: View {
    @EnvironmentObject var keychain: KeychainService

    var body: some View {
        MainView()
            .frame(minWidth: 1000, minHeight: 650)
    }
}
