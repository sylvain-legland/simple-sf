import Foundation
import SwiftUI

/// Browses and downloads MLX models from HuggingFace
@MainActor
final class HuggingFaceService: ObservableObject {
    static let shared = HuggingFaceService()

    struct HFModel: Identifiable, Hashable {
        var id: String { repoId }
        let repoId: String
        let name: String
        let params: String
        let quant: String
        let sizeGB: Double
        let downloads: Int
        let minRAMGB: Int
    }

    enum DownloadState: Equatable {
        case idle
        case downloading(progress: String)
        case completed
        case failed(String)
    }

    @Published var models: [HFModel] = []
    @Published var downloadState: DownloadState = .idle
    @Published var downloadLog: [String] = []

    private var downloadProcess: Process?

    private init() {}

    // MARK: - Recommended models per RAM tier

    static let curatedModels: [HFModel] = [
        HFModel(repoId: "mlx-community/Qwen3.5-0.8B-8bit",
                name: "Qwen 3.5 0.8B", params: "0.8B", quant: "8-bit",
                sizeGB: 1.0, downloads: 0, minRAMGB: 4),
        HFModel(repoId: "mlx-community/Qwen3.5-4B-4bit",
                name: "Qwen 3.5 4B", params: "4B", quant: "4-bit",
                sizeGB: 2.9, downloads: 0, minRAMGB: 8),
        HFModel(repoId: "mlx-community/Qwen3.5-9B-MLX-4bit",
                name: "Qwen 3.5 9B", params: "9B", quant: "4-bit",
                sizeGB: 5.6, downloads: 0, minRAMGB: 16),
        HFModel(repoId: "mlx-community/Qwen3.5-27B-4bit",
                name: "Qwen 3.5 27B", params: "27B", quant: "4-bit",
                sizeGB: 15.0, downloads: 0, minRAMGB: 24),
        HFModel(repoId: "mlx-community/Qwen3.5-35B-A3B-4bit",
                name: "Qwen 3.5 35B MoE", params: "35B (3B active)", quant: "4-bit",
                sizeGB: 19.0, downloads: 0, minRAMGB: 32),
    ]

    /// Best model that fits in the system's unified memory
    static func recommendedModel() -> HFModel {
        let ramGB = systemRAMGB()
        // Leave ~40% RAM for macOS + app overhead
        let available = Int(Double(ramGB) * 0.6)
        return curatedModels.last(where: { $0.minRAMGB <= ramGB && Int($0.sizeGB) < available })
            ?? curatedModels.first!
    }

    /// System unified memory in GB
    static func systemRAMGB() -> Int {
        var size: UInt64 = 0
        var len = MemoryLayout<UInt64>.size
        sysctlbyname("hw.memsize", &size, &len, nil, 0)
        return Int(size / 1_073_741_824)
    }

    // MARK: - Search HuggingFace API

    func searchModels(query: String = "Qwen3.5") async {
        let urlStr = "https://huggingface.co/api/models?author=mlx-community&search=\(query)&sort=downloads&direction=-1&limit=30"
        guard let url = URL(string: urlStr.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? urlStr) else { return }

        do {
            let (data, _) = try await URLSession.shared.data(from: url)
            guard let json = try JSONSerialization.jsonObject(with: data) as? [[String: Any]] else { return }

            models = json.compactMap { m in
                guard let modelId = m["modelId"] as? String else { return nil }
                let downloads = m["downloads"] as? Int ?? 0
                let parts = modelId.split(separator: "/").last.map(String.init) ?? modelId
                return HFModel(
                    repoId: modelId,
                    name: parts,
                    params: extractParams(parts),
                    quant: extractQuant(parts),
                    sizeGB: 0,
                    downloads: downloads,
                    minRAMGB: 0
                )
            }
        } catch {}
    }

    // MARK: - Download model via huggingface_hub

    func download(model: HFModel) {
        guard case .idle = downloadState else { return }
        downloadState = .downloading(progress: "Starting...")
        downloadLog = []

        let proc = Process()
        proc.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        proc.arguments = [
            "python3", "-c",
            """
            import sys
            from huggingface_hub import snapshot_download
            print(f"Downloading {'\(model.repoId)'}...", flush=True)
            path = snapshot_download(
                '\(model.repoId)',
                allow_patterns=["*.safetensors", "*.json", "tokenizer*", "*.tiktoken", "*.model"],
            )
            print(f"DONE:{path}", flush=True)
            """
        ]
        proc.environment = ProcessInfo.processInfo.environment

        let pipe = Pipe()
        let errPipe = Pipe()
        proc.standardOutput = pipe
        proc.standardError = errPipe

        pipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            guard !data.isEmpty, let line = String(data: data, encoding: .utf8) else { return }
            Task { @MainActor [weak self] in
                let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
                if trimmed.hasPrefix("DONE:") {
                    self?.downloadState = .completed
                    MLXService.shared.scanModels()
                } else if !trimmed.isEmpty {
                    self?.downloadLog.append(trimmed)
                    self?.downloadState = .downloading(progress: trimmed)
                }
            }
        }

        errPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            guard !data.isEmpty, let line = String(data: data, encoding: .utf8) else { return }
            Task { @MainActor [weak self] in
                let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
                if !trimmed.isEmpty {
                    self?.downloadLog.append(trimmed)
                    // Update progress with download percentage if found
                    if trimmed.contains("%") {
                        self?.downloadState = .downloading(progress: trimmed)
                    }
                }
            }
        }

        proc.terminationHandler = { [weak self] p in
            Task { @MainActor [weak self] in
                if p.terminationStatus != 0 {
                    if case .downloading = self?.downloadState {
                        self?.downloadState = .failed("Exit code \(p.terminationStatus)")
                    }
                }
                self?.downloadProcess = nil
            }
        }

        do {
            try proc.run()
            downloadProcess = proc
        } catch {
            downloadState = .failed(error.localizedDescription)
        }
    }

    func cancelDownload() {
        downloadProcess?.terminate()
        downloadProcess = nil
        downloadState = .idle
    }

    // MARK: - Check if model is already downloaded

    func isDownloaded(_ model: HFModel) -> Bool {
        let modelDir = model.repoId.replacingOccurrences(of: "/", with: "--")
        let home = FileManager.default.homeDirectoryForCurrentUser
        let path = home.appendingPathComponent(".cache/huggingface/hub/models--\(modelDir)/snapshots")
        return FileManager.default.fileExists(atPath: path.path)
    }

    // MARK: - Helpers

    private func extractParams(_ name: String) -> String {
        let patterns = ["0.8B", "1B", "2B", "4B", "7B", "8B", "9B", "14B", "27B", "32B", "35B", "70B", "122B", "397B"]
        return patterns.first(where: { name.contains($0) }) ?? ""
    }

    private func extractQuant(_ name: String) -> String {
        if name.contains("8bit") { return "8-bit" }
        if name.contains("6bit") { return "6-bit" }
        if name.contains("4bit") || name.contains("MXFP4") { return "4-bit" }
        return ""
    }
}
