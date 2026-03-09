import Foundation
import SwiftUI

/// Manages the local MLX LLM server (mlx_lm.server)
@MainActor
final class MLXService: ObservableObject {
    static let shared = MLXService()

    enum ServerState: Equatable {
        case stopped
        case starting
        case running(pid: Int32)
        case error(String)

        static func == (lhs: ServerState, rhs: ServerState) -> Bool {
            switch (lhs, rhs) {
            case (.stopped, .stopped): return true
            case (.starting, .starting): return true
            case (.running(let a), .running(let b)): return a == b
            case (.error(let a), .error(let b)): return a == b
            default: return false
            }
        }
    }

    struct MLXModel: Identifiable, Hashable {
        var id: String { path }
        let name: String
        let path: String
        let sizeGB: Double
        let modelType: String
    }

    @Published var state: ServerState = .stopped
    @Published var availableModels: [MLXModel] = []
    @Published var activeModel: MLXModel?
    @Published var logLines: [String] = []

    let port: Int = 8800
    private var process: Process?

    var baseURL: String { "http://127.0.0.1:\(port)/v1" }
    var isRunning: Bool {
        if case .running = state { return true }
        return false
    }

    private init() {
        scanModels()
    }

    // MARK: - Scan for installed MLX models

    func scanModels() {
        var models: [MLXModel] = []
        let home = FileManager.default.homeDirectoryForCurrentUser

        // HuggingFace hub cache — scan ALL model dirs that have snapshots with config.json
        let hubDir = home.appendingPathComponent(".cache/huggingface/hub")
        if let entries = try? FileManager.default.contentsOfDirectory(atPath: hubDir.path) {
            for entry in entries where entry.hasPrefix("models--") {
                let snapshotsDir = hubDir.appendingPathComponent(entry).appendingPathComponent("snapshots")
                guard let snapshots = try? FileManager.default.contentsOfDirectory(atPath: snapshotsDir.path),
                      let latest = snapshots.filter({ !$0.hasPrefix(".") }).sorted().last else { continue }
                let fullPath = snapshotsDir.appendingPathComponent(latest).path
                // Only include if it has a config.json (MLX model marker)
                let configPath = (fullPath as NSString).appendingPathComponent("config.json")
                guard FileManager.default.fileExists(atPath: configPath) else { continue }
                // Extract readable name
                let parts = entry.split(separator: "--", maxSplits: 2)
                let name = parts.count >= 3 ? String(parts[2]).replacingOccurrences(of: "--", with: "/") :
                           parts.count >= 2 ? String(parts[1]).replacingOccurrences(of: "--", with: "/") :
                           entry
                // Parse model type from config.json
                var modelType = ""
                if let cfgData = FileManager.default.contents(atPath: configPath),
                   let cfg = try? JSONSerialization.jsonObject(with: cfgData) as? [String: Any] {
                    modelType = cfg["model_type"] as? String ?? ""
                }
                // Compute directory size (sum of .safetensors files)
                let sizeGB = Self.directorySizeGB(fullPath)
                models.append(MLXModel(name: name, path: fullPath, sizeGB: sizeGB, modelType: modelType))
            }
        }

        // Direct ~/.cache/mlx-models/ directory
        let mlxDir = home.appendingPathComponent(".cache/mlx-models")
        if let entries = try? FileManager.default.contentsOfDirectory(atPath: mlxDir.path) {
            for entry in entries {
                let fullPath = mlxDir.appendingPathComponent(entry).path
                var isDir: ObjCBool = false
                if FileManager.default.fileExists(atPath: fullPath, isDirectory: &isDir), isDir.boolValue {
                    let sizeGB = Self.directorySizeGB(fullPath)
                    models.append(MLXModel(name: entry, path: fullPath, sizeGB: sizeGB, modelType: ""))
                }
            }
        }

        availableModels = models
        // Default to Qwen3.5 if available
        if activeModel == nil {
            activeModel = models.first(where: { $0.name.lowercased().contains("qwen3.5") })
                ?? models.first
        }
    }

    // MARK: - Directory size

    private static func directorySizeGB(_ path: String) -> Double {
        let fm = FileManager.default
        guard let enumerator = fm.enumerator(atPath: path) else { return 0 }
        var total: UInt64 = 0
        while let file = enumerator.nextObject() as? String {
            let full = (path as NSString).appendingPathComponent(file)
            if let attrs = try? fm.attributesOfItem(atPath: full),
               let size = attrs[.size] as? UInt64 {
                total += size
            }
        }
        return Double(total) / 1_073_741_824.0
    }

    // MARK: - Start server

    func start(model: MLXModel? = nil) {
        guard !isRunning else { return }
        let chosen = model ?? activeModel
        guard let chosen else {
            state = .error("No MLX model selected")
            return
        }

        // Stop Ollama if running — only one local LLM at a time
        if OllamaService.shared.isRunning {
            OllamaService.shared.stop()
        }

        // Kill any existing mlx_lm server processes to prevent duplicates
        Self.killExistingMLXServers()

        activeModel = chosen
        state = .starting
        logLines = []

        // Resolve python3 path — /usr/bin/env may not find homebrew python in codesigned apps
        let python3Path = Self.findPython3()

        let proc = Process()
        proc.executableURL = URL(fileURLWithPath: python3Path)
        proc.arguments = [
            "-m", "mlx_lm", "server",
            "--model", chosen.path,
            "--host", "127.0.0.1",
            "--port", String(port)
        ]
        // Ensure homebrew paths are in PATH for the subprocess
        var env = ProcessInfo.processInfo.environment
        let extraPaths = ["/opt/homebrew/bin", "/usr/local/bin", "/opt/homebrew/sbin"]
        let existingPath = env["PATH"] ?? "/usr/bin:/bin"
        env["PATH"] = (extraPaths + [existingPath]).joined(separator: ":")
        proc.environment = env

        let pipe = Pipe()
        let errPipe = Pipe()
        proc.standardOutput = pipe
        proc.standardError = errPipe

        // Read stderr for startup logs
        errPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            guard !data.isEmpty, let line = String(data: data, encoding: .utf8) else { return }
            Task { @MainActor [weak self] in
                let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
                if !trimmed.isEmpty {
                    self?.logLines.append(trimmed)
                    if self?.logLines.count ?? 0 > 50 {
                        self?.logLines.removeFirst()
                    }
                }
            }
        }

        proc.terminationHandler = { [weak self] p in
            Task { @MainActor [weak self] in
                if case .running = self?.state {
                    self?.state = .stopped
                }
                self?.process = nil
            }
        }

        do {
            try proc.run()
            process = proc
            // Poll for server readiness
            Task {
                var ready = false
                for _ in 0..<30 {
                    try? await Task.sleep(nanoseconds: 1_000_000_000)
                    if await checkHealth() {
                        ready = true
                        break
                    }
                }
                if ready {
                    state = .running(pid: proc.processIdentifier)
                } else if proc.isRunning {
                    state = .running(pid: proc.processIdentifier)
                } else {
                    state = .error("Server failed to start")
                }
            }
        } catch {
            state = .error(error.localizedDescription)
        }
    }

    // MARK: - Stop server

    func stop() {
        if let proc = process, proc.isRunning {
            proc.terminate()
        }
        process = nil
        state = .stopped
    }

    /// Kill any stale mlx_lm server processes to prevent duplicates
    private static func killExistingMLXServers() {
        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/usr/bin/pkill")
        task.arguments = ["-f", "mlx_lm.*server"]
        try? task.run()
        task.waitUntilExit()
    }

    /// Find python3 binary — prefer homebrew, fall back to system
    private static func findPython3() -> String {
        let candidates = [
            "/opt/homebrew/bin/python3",
            "/usr/local/bin/python3",
            "/usr/bin/python3"
        ]
        for path in candidates {
            if FileManager.default.fileExists(atPath: path) {
                return path
            }
        }
        return "/usr/bin/env"  // last resort
    }

    // MARK: - Health check

    private nonisolated func checkHealth() async -> Bool {
        let url = URL(string: "http://127.0.0.1:8800/v1/models")!
        var req = URLRequest(url: url)
        req.timeoutInterval = 2
        do {
            let (_, response) = try await URLSession.shared.data(for: req)
            return (response as? HTTPURLResponse)?.statusCode == 200
        } catch {
            return false
        }
    }

    // MARK: - List models from running server

    nonisolated func fetchServerModels() async -> [String] {
        let url = URL(string: "http://127.0.0.1:8800/v1/models")!
        do {
            let (data, _) = try await URLSession.shared.data(from: url)
            if let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
               let models = json["data"] as? [[String: Any]] {
                return models.compactMap { $0["id"] as? String }
            }
        } catch {}
        return []
    }
}
