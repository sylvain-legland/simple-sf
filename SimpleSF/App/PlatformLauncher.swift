import Foundation
import os.log

private let log = Logger(subsystem: "com.macaron.simple-sf", category: "PlatformLauncher")

@MainActor
final class PlatformLauncher: ObservableObject {
    static let shared = PlatformLauncher()

    @Published var state: LaunchState = .idle
    @Published var port: Int = 0
    @Published var logLines: [String] = []

    private var process: Process?

    enum LaunchState {
        case idle, starting, ready, failed(String), stopped
    }

    private init() {}

    func start() async {
        guard case .idle = state else { return }
        state = .starting

        // Locate the embedded Rust server binary
        guard let serverPath = Bundle.main.path(forResource: "simple-sf-server", ofType: nil) ??
              findServerBinary() else {
            state = .failed("simple-sf-server binary not found in app bundle.")
            return
        }

        let freePort = findFreePort()
        port = freePort

        let proc = Process()
        proc.executableURL = URL(fileURLWithPath: serverPath)
        proc.arguments = []

        var env = ProcessInfo.processInfo.environment
        env["PORT"] = String(freePort)
        env["SF_DATA_DIR"] = dataDirectory()
        env["JWT_SECRET"] = "simple-sf-\(ProcessInfo.processInfo.hostName)-2026"
        // Inject LLM keys from Keychain
        for provider in LLMProvider.allCases {
            if let key = KeychainStore.shared.getKey(for: provider) {
                env[provider.envVar] = key
            }
        }
        proc.environment = env

        let pipe = Pipe()
        proc.standardOutput = pipe
        proc.standardError = pipe
        pipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            guard !data.isEmpty, let line = String(data: data, encoding: .utf8) else { return }
            Task { @MainActor [weak self] in
                self?.logLines.append(contentsOf: line.components(separatedBy: "\n").filter { !$0.isEmpty })
                if self?.logLines.count ?? 0 > 500 { self?.logLines.removeFirst(100) }
            }
        }

        do {
            try proc.run()
            self.process = proc
        } catch {
            state = .failed("Failed to launch server: \(error.localizedDescription)")
            return
        }

        // Poll until server is up (max 10s — Rust starts instantly)
        for _ in 0..<20 {
            try? await Task.sleep(nanoseconds: 500_000_000)
            if await isServerUp(port: freePort) {
                state = .ready
                log.info("SF Rust server ready on port \(freePort)")
                return
            }
        }
        state = .failed("Server did not respond within 10 seconds")
    }

    func stop() {
        process?.terminate()
        process = nil
        state = .stopped
    }

    func setLLMKey(_ key: String, for provider: LLMProvider) {
        KeychainStore.shared.setKey(key, for: provider)
        // If server already running, restart to pick up new key
        if case .ready = state {
            stop()
            Task { @MainActor in
                self.state = .idle
                await self.start()
            }
        }
    }

    private func isServerUp(port: Int) async -> Bool {
        guard let url = URL(string: "http://127.0.0.1:\(port)/health") else { return false }
        do {
            let (_, response) = try await URLSession.shared.data(from: url)
            return (response as? HTTPURLResponse)?.statusCode == 200
        } catch { return false }
    }

    /// Fallback: look next to the Swift binary (dev mode)
    private func findServerBinary() -> String? {
        let exe = Bundle.main.executableURL?.deletingLastPathComponent()
        let candidate = exe?.appendingPathComponent("simple-sf-server").path
        if let c = candidate, FileManager.default.fileExists(atPath: c) { return c }
        return nil
    }

    private func findFreePort() -> Int {
        let sock = socket(AF_INET, SOCK_STREAM, 0)
        defer { close(sock) }
        var addr = sockaddr_in()
        addr.sin_family = sa_family_t(AF_INET)
        addr.sin_addr.s_addr = INADDR_ANY
        addr.sin_port = 0
        var len = socklen_t(MemoryLayout<sockaddr_in>.size)
        withUnsafeMutablePointer(to: &addr) { ptr in
            ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                bind(sock, $0, socklen_t(MemoryLayout<sockaddr_in>.size))
            }
            ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                getsockname(sock, $0, &len)
            }
        }
        return Int(addr.sin_port.bigEndian)
    }

    private func dataDirectory() -> String {
        let app = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let dir = app.appendingPathComponent("SimpleSF/data")
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.path
    }
}
