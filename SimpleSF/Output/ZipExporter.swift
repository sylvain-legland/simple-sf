import Foundation
import AppKit

final class ZipExporter {
    static func export(projectId: String, name: String) async throws -> URL {
        let workspaceURL = workspaceURL(for: projectId)
        guard FileManager.default.fileExists(atPath: workspaceURL.path) else {
            throw ExportError.workspaceNotFound(workspaceURL.path)
        }

        let panel = await NSSavePanel.run {
            $0.nameFieldStringValue = "\(name).zip"
            $0.allowedContentTypes = [.zip]
            $0.prompt = "Export"
        }

        guard let destURL = panel else {
            throw ExportError.cancelled
        }

        // Create zip using Process (no external deps)
        let proc = Process()
        proc.executableURL = URL(fileURLWithPath: "/usr/bin/zip")
        proc.arguments = ["-r", destURL.path, "."]
        proc.currentDirectoryURL = workspaceURL

        let pipe = Pipe()
        proc.standardOutput = pipe
        proc.standardError = pipe

        try proc.run()
        proc.waitUntilExit()

        guard proc.terminationStatus == 0 else {
            let output = String(data: pipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
            throw ExportError.zipFailed(output)
        }

        return destURL
    }

    private static func workspaceURL(for projectId: String) -> URL {
        let appSupport = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        return appSupport.appendingPathComponent("SimpleSF/data/workspaces/\(projectId)")
    }

    enum ExportError: LocalizedError {
        case workspaceNotFound(String)
        case cancelled
        case zipFailed(String)

        var errorDescription: String? {
            switch self {
            case .workspaceNotFound(let p): return "Workspace not found: \(p)"
            case .cancelled:               return "Export cancelled"
            case .zipFailed(let out):      return "zip failed: \(out)"
            }
        }
    }
}

extension NSSavePanel {
    @MainActor
    static func run(_ configure: (NSSavePanel) -> Void) async -> URL? {
        let panel = NSSavePanel()
        configure(panel)
        let result = await withCheckedContinuation { continuation in
            panel.begin { response in
                continuation.resume(returning: response == .OK ? panel.url : nil)
            }
        }
        return result
    }
}
