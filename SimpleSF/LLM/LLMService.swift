import Foundation

// MARK: - Provider definitions

enum LLMProvider: String, CaseIterable, Codable {
    case ollama, mlx, openai, anthropic, gemini, minimax, kimi, openrouter, alibaba, glm

    var displayName: String {
        switch self {
        case .ollama:     return "Ollama"
        case .mlx:        return "Apple MLX"
        case .openai:     return "OpenAI"
        case .anthropic:  return "Anthropic"
        case .gemini:     return "Google Gemini"
        case .minimax:    return "MiniMax"
        case .kimi:       return "Kimi (Moonshot)"
        case .openrouter: return "OpenRouter"
        case .alibaba:    return "Alibaba Qwen"
        case .glm:        return "Zhipu GLM"
        }
    }

    var subtitle: String {
        switch self {
        case .ollama:     return "Local · llama.cpp engine"
        case .mlx:        return "Local · Apple Silicon optimized"
        case .openai:     return "Cloud · GPT-4o, o1, o3"
        case .anthropic:  return "Cloud · Claude Sonnet, Opus"
        case .gemini:     return "Cloud · Gemini 2.0 Flash/Pro"
        case .minimax:    return "Cloud · MiniMax-M2.5"
        case .kimi:       return "Cloud · Moonshot"
        case .openrouter: return "Cloud · Multi-model gateway"
        case .alibaba:    return "Cloud · Qwen-Turbo/Max"
        case .glm:        return "Cloud · GLM-4 Flash"
        }
    }

    var envVar: String {
        switch self {
        case .ollama:     return "OLLAMA_LOCAL"
        case .mlx:        return "MLX_LOCAL"
        case .openai:     return "OPENAI_API_KEY"
        case .anthropic:  return "ANTHROPIC_API_KEY"
        case .gemini:     return "GEMINI_API_KEY"
        case .minimax:    return "MINIMAX_API_KEY"
        case .kimi:       return "KIMI_API_KEY"
        case .openrouter: return "OPENROUTER_API_KEY"
        case .alibaba:    return "ALIBABA_API_KEY"
        case .glm:        return "GLM_API_KEY"
        }
    }

    var baseURL: String {
        switch self {
        case .ollama:     return "http://127.0.0.1:11434/v1"
        case .mlx:        return "http://127.0.0.1:8800/v1"
        case .openai:     return "https://api.openai.com/v1"
        case .anthropic:  return "https://api.anthropic.com/v1"
        case .gemini:     return "https://generativelanguage.googleapis.com/v1beta"
        case .minimax:    return "https://api.minimax.io/v1"
        case .kimi:       return "https://api.moonshot.cn/v1"
        case .openrouter: return "https://openrouter.ai/api/v1"
        case .alibaba:    return "https://dashscope.aliyuncs.com/compatible-mode/v1"
        case .glm:        return "https://open.bigmodel.cn/api/paas/v4"
        }
    }

    var defaultModel: String {
        switch self {
        case .ollama:     return "qwen3:14b"
        case .mlx:        return "mlx-local"
        case .openai:     return "gpt-4o-mini"
        case .anthropic:  return "claude-3-5-haiku-20241022"
        case .gemini:     return "gemini-2.0-flash"
        case .minimax:    return "MiniMax-M2.5"
        case .kimi:       return "moonshot-v1-8k"
        case .openrouter: return "openai/gpt-4o-mini"
        case .alibaba:    return "qwen-turbo"
        case .glm:        return "glm-4-flash"
        }
    }

    var docURL: String {
        switch self {
        case .ollama:     return "https://ollama.com/library"
        case .mlx:        return "https://github.com/ml-explore/mlx-lm"
        case .openai:     return "https://platform.openai.com/api-keys"
        case .anthropic:  return "https://console.anthropic.com/settings/keys"
        case .gemini:     return "https://aistudio.google.com/app/apikey"
        case .minimax:    return "https://platform.minimaxi.com/user-center/basic-information/interface-key"
        case .kimi:       return "https://platform.moonshot.cn/console/api-keys"
        case .openrouter: return "https://openrouter.ai/keys"
        case .alibaba:    return "https://bailian.console.aliyun.com/#/api-key"
        case .glm:        return "https://open.bigmodel.cn/usercenter/apikeys"
        }
    }

    var isLocal: Bool { self == .mlx || self == .ollama }

    /// Cloud providers that need API keys
    static var cloudProviders: [LLMProvider] {
        allCases.filter { !$0.isLocal }
    }

    /// Local providers
    static var localProviders: [LLMProvider] {
        allCases.filter { $0.isLocal }
    }
}

// MARK: - Message

struct LLMMessage: Codable {
    var role: String    // "user" | "assistant" | "system"
    var content: String
    // Agent metadata (populated for discussion messages)
    var agentId: String?
    var agentName: String?
    var agentRole: String?
    var messageType: String?
    var toAgents: [String]?

    var isAgentMessage: Bool { agentId != nil }
}

// MARK: - LLMService (direct calls, no server)

@MainActor
final class LLMService: ObservableObject {
    static let shared = LLMService()
    private init() {}

    // Active provider: respects explicit user selection, then auto-detect
    var activeProvider: LLMProvider? {
        // Explicit selection — trust it even if keychain scan isn't done yet
        if let sel = AppState.shared.selectedProvider {
            switch sel {
            case .mlx where MLXService.shared.isRunning: return .mlx
            case .ollama where OllamaService.shared.isRunning: return .ollama
            case let p where !p.isLocal:
                // Trust user's explicit cloud selection; don't require storedProviders check
                return p
            default: break // local provider selected but not running
            }
        }
        // Auto-detect
        let pref = AppState.shared.preferredLocalProvider
        if pref == "mlx" && MLXService.shared.isRunning { return .mlx }
        if pref == "ollama" && OllamaService.shared.isRunning { return .ollama }
        if MLXService.shared.isRunning { return .mlx }
        if OllamaService.shared.isRunning { return .ollama }
        return LLMProvider.cloudProviders.first { KeychainService.shared.storedProviders.contains($0) }
    }

    /// Human-readable provider + model for UI display
    var activeDisplayName: String {
        guard let prov = activeProvider else { return "No LLM configured" }
        switch prov {
        case .mlx:
            let model = MLXService.shared.activeModel?.name ?? "loading…"
            let short = model.split(separator: "/").last.map(String.init) ?? model
            return "Apple MLX · \(short)"
        case .ollama:
            let model = OllamaService.shared.activeModel?.name ?? "loading…"
            return "Ollama · \(model)"
        default:
            return prov.displayName
        }
    }

    // MARK: - One-shot completion

    func complete(messages: [LLMMessage], system: String? = nil, provider: LLMProvider? = nil) async throws -> String {
        let prov = provider ?? activeProvider
        guard let prov else { throw LLMError.noProvider }
        let key: String
        if prov.isLocal {
            key = "no-key"
        } else {
            guard let k = KeychainService.shared.key(for: prov) else { throw LLMError.noKey(prov) }
            key = k
        }

        var all: [[String: String]] = []
        if let sys = system { all.append(["role": "system", "content": sys]) }
        all += messages.map { ["role": $0.role, "content": $0.content] }

        let body: [String: Any] = [
            "model": prov == .ollama ? (OllamaService.shared.activeModel?.name ?? prov.defaultModel) :
                     prov == .mlx ? (MLXService.shared.activeModel?.name ?? prov.defaultModel) :
                     prov.defaultModel,
            "messages": all,
            "max_tokens": 4096,
            "temperature": 0.7
        ]

        let url = URL(string: "\(prov.baseURL)/chat/completions")!
        var req = URLRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.setValue("Bearer \(key)", forHTTPHeaderField: "Authorization")
        if prov == .anthropic {
            req.setValue(key, forHTTPHeaderField: "x-api-key")
            req.setValue("2023-06-01", forHTTPHeaderField: "anthropic-version")
            req.setValue(nil, forHTTPHeaderField: "Authorization")
        }
        req.httpBody = try JSONSerialization.data(withJSONObject: body)
        req.timeoutInterval = 60

        let (data, response) = try await URLSession.shared.data(for: req)
        guard let http = response as? HTTPURLResponse, http.statusCode < 300 else {
            let text = String(data: data, encoding: .utf8) ?? "unknown"
            throw LLMError.api(text)
        }

        let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        guard let content = ((json?["choices"] as? [[String: Any]])?.first?["message"] as? [String: Any])?["content"] as? String else {
            throw LLMError.parse
        }
        return stripThinking(content)
    }

    // MARK: - Streaming via AsyncStream

    func stream(messages: [LLMMessage], system: String? = nil) -> AsyncStream<String> {
        AsyncStream { continuation in
            Task {
                do {
                    let prov = self.activeProvider
                    guard let prov else { throw LLMError.noProvider }
                    let key: String
                    if prov.isLocal {
                        key = "no-key"
                    } else {
                        guard let k = KeychainService.shared.key(for: prov) else { throw LLMError.noKey(prov) }
                        key = k
                    }

                    var all: [[String: String]] = []
                    if let sys = system { all.append(["role": "system", "content": sys]) }
                    all += messages.map { ["role": $0.role, "content": $0.content] }

                    let body: [String: Any] = [
                        "model": prov == .ollama ? (OllamaService.shared.activeModel?.name ?? prov.defaultModel) :
                                 prov == .mlx ? (MLXService.shared.activeModel?.name ?? prov.defaultModel) :
                                 prov.defaultModel,
                        "messages": all,
                        "max_tokens": 4096,
                        "temperature": 0.7,
                        "stream": true
                    ]

                    let url = URL(string: "\(prov.baseURL)/chat/completions")!
                    var req = URLRequest(url: url)
                    req.httpMethod = "POST"
                    req.setValue("application/json", forHTTPHeaderField: "Content-Type")
                    req.setValue("Bearer \(key)", forHTTPHeaderField: "Authorization")
                    req.httpBody = try JSONSerialization.data(withJSONObject: body)
                    req.timeoutInterval = 120

                    let (bytes, _) = try await URLSession.shared.bytes(for: req)
                    var buffer = ""
                    for try await line in bytes.lines {
                        guard line.hasPrefix("data: ") else { continue }
                        let chunk = String(line.dropFirst(6))
                        if chunk == "[DONE]" { break }
                        guard let data = chunk.data(using: .utf8),
                              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                              let delta = ((json["choices"] as? [[String: Any]])?.first?["delta"] as? [String: Any])?["content"] as? String
                        else { continue }
                        buffer += delta
                        // Strip think blocks incrementally
                        if !buffer.contains("<think>") || buffer.contains("</think>") {
                            let stripped = stripThinking(buffer)
                            if stripped != buffer {
                                continuation.yield(stripped)
                                buffer = ""
                            } else {
                                continuation.yield(delta)
                            }
                        }
                    }
                    continuation.finish()
                } catch {
                    continuation.yield("\n[Error: \(error.localizedDescription)]")
                    continuation.finish()
                }
            }
        }
    }

    // MARK: - Test connection

    func testConnection(provider: LLMProvider, key: String) async -> Bool {
        if provider == .ollama {
            await OllamaService.shared.refresh()
            return OllamaService.shared.isRunning
        }
        if provider == .mlx {
            return await MLXService.shared.fetchServerModels().count > 0
        }
        let body: [String: Any] = [
            "model": provider.defaultModel,
            "messages": [["role": "user", "content": "hi"]],
            "max_tokens": 5
        ]
        let url = URL(string: "\(provider.baseURL)/chat/completions")!
        var req = URLRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.setValue("Bearer \(key)", forHTTPHeaderField: "Authorization")
        if provider == .anthropic {
            req.setValue(key, forHTTPHeaderField: "x-api-key")
            req.setValue("2023-06-01", forHTTPHeaderField: "anthropic-version")
            req.setValue(nil, forHTTPHeaderField: "Authorization")
        }
        req.httpBody = try? JSONSerialization.data(withJSONObject: body)
        req.timeoutInterval = 15

        do {
            let (_, response) = try await URLSession.shared.data(for: req)
            return (response as? HTTPURLResponse)?.statusCode ?? 999 < 400
        } catch {
            return false
        }
    }

    // MARK: - Helpers

    private func stripThinking(_ s: String) -> String {
        var out = s
        while let start = out.range(of: "<think>"), let end = out.range(of: "</think>") {
            if start.lowerBound <= end.lowerBound {
                out.removeSubrange(start.lowerBound..<end.upperBound)
            } else { break }
        }
        return out.trimmingCharacters(in: .whitespacesAndNewlines)
    }
}

enum LLMError: LocalizedError {
    case noProvider, noKey(LLMProvider), api(String), parse

    var errorDescription: String? {
        switch self {
        case .noProvider: return "No LLM provider configured. Add an API key in Settings."
        case .noKey(let p): return "No API key for \(p.displayName). Add it in Settings."
        case .api(let msg): return "API error: \(msg)"
        case .parse: return "Could not parse LLM response"
        }
    }
}
