import Foundation

// Ref: FT-SSF-016
struct GitConfig {
    var repoURL: String        // e.g. https://github.com/user/repo.git
    var branch: String = "main"
    var token: String          // GitHub PAT or GitLab token
    var commitMessage: String = "feat: update from Simple SF"
}

final class GitPusher {
    static func push(projectId: String, config: GitConfig) async throws {
        let workspaceURL = workspaceURL(for: projectId)
        guard FileManager.default.fileExists(atPath: workspaceURL.path) else {
            throw GitError.workspaceNotFound
        }

        // Inject token into URL for HTTPS auth
        let authenticatedURL = injectToken(config.token, into: config.repoURL)

        let cmds: [(String, [String])] = [
            ("/usr/bin/git", ["init"]),
            ("/usr/bin/git", ["config", "user.email", "simpleSF@macaron.ai"]),
            ("/usr/bin/git", ["config", "user.name", "Simple SF"]),
            ("/usr/bin/git", ["checkout", "-B", config.branch]),
            ("/usr/bin/git", ["add", "."]),
            ("/usr/bin/git", ["commit", "-m", config.commitMessage, "--allow-empty"]),
            ("/usr/bin/git", ["remote", "remove", "origin"]),  // ignore error
            ("/usr/bin/git", ["remote", "add", "origin", authenticatedURL]),
            ("/usr/bin/git", ["push", "-u", "origin", config.branch, "--force"]),
        ]

        for (exe, args) in cmds {
            let result = try await run(exe: exe, args: args, cwd: workspaceURL)
            if result.status != 0 {
                // "remote remove" can fail if no remote — that's OK
                let isOptional = args.contains("remove")
                if !isOptional {
                    throw GitError.commandFailed(args.joined(separator: " "), result.output)
                }
            }
        }
    }

    private static func injectToken(_ token: String, into repoURL: String) -> String {
        guard !token.isEmpty else { return repoURL }
        // https://github.com/user/repo → https://token@github.com/user/repo
        if repoURL.hasPrefix("https://") {
            return repoURL.replacingOccurrences(of: "https://", with: "https://\(token)@")
        }
        return repoURL
    }

    private static func run(exe: String, args: [String], cwd: URL) async throws -> (status: Int32, output: String) {
        let proc = Process()
        proc.executableURL = URL(fileURLWithPath: exe)
        proc.arguments = args
        proc.currentDirectoryURL = cwd

        let pipe = Pipe()
        proc.standardOutput = pipe
        proc.standardError = pipe

        try proc.run()
        proc.waitUntilExit()

        let output = String(data: pipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
        return (proc.terminationStatus, output)
    }

    private static func workspaceURL(for projectId: String) -> URL {
        let appSupport = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        return appSupport.appendingPathComponent("SimpleSF/data/workspaces/\(projectId)")
    }

    enum GitError: LocalizedError {
        case workspaceNotFound
        case commandFailed(String, String)

        var errorDescription: String? {
            switch self {
            case .workspaceNotFound:            return "Project workspace not found"
            case .commandFailed(let cmd, let out): return "git \(cmd) failed:\n\(out)"
            }
        }
    }
}
