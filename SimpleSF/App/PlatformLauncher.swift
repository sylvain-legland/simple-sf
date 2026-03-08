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

        guard let pythonPath = Bundle.main.path(forResource: "Python.framework/Versions/3.12/bin/python3", ofType: nil),
              let platformDir = Bundle.main.path(forResource: "platform", ofType: nil),
              let sitePackages = Bundle.main.path(forResource: "site-packages", ofType: nil) else {
            state = .failed("Python runtime not found. Run Scripts/embed_python.sh first.")
            return
        }

        let freePort = findFreePort()
        port = freePort

        let proc = Process()
        proc.executableURL = URL(fileURLWithPath: pythonPath)
        proc.arguments = [
            "-m", "uvicorn",
            "platform.server:app",
            "--host", "127.0.0.1",
            "--port", String(freePort),
            "--ws", "none",
            "--log-level", "warning"
        ]

        var env = ProcessInfo.processInfo.environment
        env["PYTHONPATH"] = sitePackages
        env["SF_DATA_DIR"] = dataDirectory()
        env["SF_DEMO_PASSWORD"] = "demo2026"
        // Inject LLM keys from Keychain
        for provider in LLMProvider.allCases {
            if let key = KeychainStore.shared.getKey(for: provider) {
                env[provider.envVar] = key
            }
        }
        proc.environment = env
        proc.currentDirectoryURL = URL(fileURLWithPath: platformDir).deletingLastPathComponent()

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
            state = .failed("Failed to launch Python: \(error)")
            return
        }

        // Poll until server is up (max 30s)
        for _ in 0..<60 {
            try? await Task.sleep(nanoseconds: 500_000_000)
            if await isServerUp(port: freePort) {
                state = .ready
                log.info("SF server ready on port \(freePort)")
                return
            }
        }
        state = .failed("Server did not start within 30 seconds")
    }

    func stop() {
        process?.terminate()
        process = nil
        state = .stopped
    }

    private func isServerUp(port: Int) async -> Bool {
        guard let url = URL(string: "http://127.0.0.1:\(port)/api/health") else { return false }
        do {
            let (_, response) = try await URLSession.shared.data(from: url)
            return (response as? HTTPURLResponse)?.statusCode == 200
        } catch { return false }
    }

    private func findFreePort() -> Int {
        // Bind to port 0 and let OS assign a free port
        let sock = socket(AF_INET, SOCK_STREAM, 0)
        defer { close(sock) }
        var addr = sockaddr_in()
        addr.sin_family = sa_family_t(AF_INET)
        addr.sin_addr.s_addr = INADDR_ANY
        addr.sin_port = 0
        var len = socklen_t(MemoryLayout<sockaddr_in>.size)
        withUnsafeMutablePointer(to: &addr) { ptr in
            ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) { bind(sock, $0, socklen_t(MemoryLayout<sockaddr_in>.size)) }
            ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) { getsockname(sock, $0, &len) }
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
