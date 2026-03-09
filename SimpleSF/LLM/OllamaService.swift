import Foundation
import SwiftUI

/// Detects and manages Ollama (localhost:11434)
@MainActor
final class OllamaService: ObservableObject {
    static let shared = OllamaService()

    struct OllamaModel: Identifiable, Hashable {
        var id: String { name }
        let name: String
        let size: String
        let family: String
        let quantization: String
    }

    @Published var isRunning = false
    @Published var availableModels: [OllamaModel] = []
    @Published var activeModel: OllamaModel?

    let port: Int = 11434
    var baseURL: String { "http://127.0.0.1:\(port)" }
    var openaiBaseURL: String { "\(baseURL)/v1" }

    private init() {
        Task { await refresh() }
    }

    // MARK: - Refresh: check if Ollama is running + list models

    func refresh() async {
        let url = URL(string: "\(baseURL)/api/tags")!
        var req = URLRequest(url: url)
        req.timeoutInterval = 3

        do {
            let (data, response) = try await URLSession.shared.data(for: req)
            guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                isRunning = false
                availableModels = []
                return
            }
            isRunning = true

            if let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
               let models = json["models"] as? [[String: Any]] {
                availableModels = models.compactMap { m in
                    guard let name = m["name"] as? String else { return nil }
                    let details = m["details"] as? [String: Any]
                    let size = details?["parameter_size"] as? String ?? ""
                    let family = details?["family"] as? String ?? ""
                    let quant = details?["quantization_level"] as? String ?? ""
                    return OllamaModel(name: name, size: size, family: family, quantization: quant)
                }
                if activeModel == nil {
                    activeModel = availableModels.first
                }
            }
        } catch {
            isRunning = false
            availableModels = []
        }
    }

    // MARK: - Start Ollama (open the app)

    func start() {
        let appURL = URL(fileURLWithPath: "/Applications/Ollama.app")
        if FileManager.default.fileExists(atPath: appURL.path) {
            NSWorkspace.shared.openApplication(at: appURL, configuration: .init())
            // Poll until ready
            Task {
                for _ in 0..<15 {
                    try? await Task.sleep(nanoseconds: 2_000_000_000)
                    await refresh()
                    if isRunning { break }
                }
            }
        }
    }

    // MARK: - Pull a model

    func pull(model: String) async {
        let url = URL(string: "\(baseURL)/api/pull")!
        var req = URLRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.httpBody = try? JSONSerialization.data(withJSONObject: ["name": model])
        req.timeoutInterval = 600
        _ = try? await URLSession.shared.data(for: req)
        await refresh()
    }

    // MARK: - Stop Ollama

    func stop() {
        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/usr/bin/pkill")
        task.arguments = ["-f", "ollama serve"]
        try? task.run()
        task.waitUntilExit()
        isRunning = false
        availableModels = []
    }
}
