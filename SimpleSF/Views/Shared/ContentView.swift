import SwiftUI

struct ContentView: View {
    @EnvironmentObject var launcher: PlatformLauncher
    @EnvironmentObject var appState: AppState

    var body: some View {
        Group {
            switch launcher.state {
            case .idle, .starting:
                LaunchScreen()
            case .ready where !appState.isOnboarded:
                OnboardingView()
            case .ready:
                MainView()
            case .failed(let msg):
                ErrorScreen(message: msg)
            case .stopped:
                LaunchScreen()
            }
        }
        .frame(minWidth: 900, minHeight: 620)
    }
}

struct LaunchScreen: View {
    @EnvironmentObject var launcher: PlatformLauncher

    var body: some View {
        VStack(spacing: 24) {
            Image(systemName: "cpu")
                .resizable().scaledToFit().frame(width: 64)
                .foregroundStyle(.purple)
            Text("Simple SF").font(.largeTitle.bold())
            Text("Starting embedded platform...")
                .foregroundStyle(.secondary)
            ProgressView().scaleEffect(1.2)
            if !launcher.logLines.isEmpty {
                Text(launcher.logLines.last ?? "")
                    .font(.caption.monospaced())
                    .foregroundStyle(.tertiary)
                    .lineLimit(1)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(.background)
    }
}

struct ErrorScreen: View {
    let message: String
    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "exclamationmark.triangle.fill")
                .resizable().scaledToFit().frame(width: 56)
                .foregroundStyle(.red)
            Text("Launch Failed").font(.title.bold())
            Text(message)
                .font(.body).foregroundStyle(.secondary)
                .multilineTextAlignment(.center).frame(maxWidth: 400)
            Button("Run embed_python.sh") {
                NSWorkspace.shared.open(URL(fileURLWithPath: "Scripts/embed_python.sh"))
            }.buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(40)
    }
}
