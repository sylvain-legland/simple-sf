import Foundation
import Security

enum LLMProvider: String, CaseIterable, Identifiable {
    case openrouter, openai, anthropic, gemini, kimi, minimax, qwen, glm

    var id: String { rawValue }

    var displayName: String {
        switch self {
        case .openrouter: return "OpenRouter"
        case .openai:     return "OpenAI"
        case .anthropic:  return "Anthropic"
        case .gemini:     return "Google Gemini"
        case .kimi:       return "Kimi (Moonshot)"
        case .minimax:    return "MiniMax"
        case .qwen:       return "Alibaba Qwen"
        case .glm:        return "Zhipu GLM"
        }
    }

    var baseURL: String {
        switch self {
        case .openrouter: return "https://openrouter.ai/api/v1"
        case .openai:     return "https://api.openai.com/v1"
        case .anthropic:  return "https://api.anthropic.com/v1"
        case .gemini:     return "https://generativelanguage.googleapis.com/v1beta"
        case .kimi:       return "https://api.moonshot.cn/v1"
        case .minimax:    return "https://api.minimax.io/v1"
        case .qwen:       return "https://dashscope.aliyuncs.com/compatible-mode/v1"
        case .glm:        return "https://open.bigmodel.cn/api/paas/v4"
        }
    }

    var envVar: String {
        switch self {
        case .openrouter: return "OPENROUTER_API_KEY"
        case .openai:     return "OPENAI_API_KEY"
        case .anthropic:  return "ANTHROPIC_API_KEY"
        case .gemini:     return "GOOGLE_API_KEY"
        case .kimi:       return "KIMI_API_KEY"
        case .minimax:    return "MINIMAX_API_KEY"
        case .qwen:       return "DASHSCOPE_API_KEY"
        case .glm:        return "GLM_API_KEY"
        }
    }

    var keyPlaceholder: String { "sk-..." }

    var docURL: URL? { URL(string: docsURLString) }
    private var docsURLString: String {
        switch self {
        case .openrouter: return "https://openrouter.ai/keys"
        case .openai:     return "https://platform.openai.com/api-keys"
        case .anthropic:  return "https://console.anthropic.com/keys"
        case .gemini:     return "https://aistudio.google.com/app/apikey"
        case .kimi:       return "https://platform.moonshot.cn/console/api-keys"
        case .minimax:    return "https://www.minimaxi.com/account/basic-information"
        case .qwen:       return "https://bailian.console.aliyun.com/"
        case .glm:        return "https://open.bigmodel.cn/usercenter/apikeys"
        }
    }
}

@MainActor
final class KeychainStore {
    static let shared = KeychainStore()
    private let service = "com.macaron.simple-sf"
    private init() {}

    func setKey(_ key: String, for provider: LLMProvider) {
        let data = Data(key.utf8)
        let query: [CFString: Any] = [
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: service,
            kSecAttrAccount: provider.rawValue,
            kSecValueData: data
        ]
        SecItemDelete(query as CFDictionary)
        SecItemAdd(query as CFDictionary, nil)
    }

    func getKey(for provider: LLMProvider) -> String? {
        let query: [CFString: Any] = [
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: service,
            kSecAttrAccount: provider.rawValue,
            kSecReturnData: true,
            kSecMatchLimit: kSecMatchLimitOne
        ]
        var result: AnyObject?
        guard SecItemCopyMatching(query as CFDictionary, &result) == errSecSuccess,
              let data = result as? Data else { return nil }
        return String(data: data, encoding: .utf8)
    }

    func deleteKey(for provider: LLMProvider) {
        let query: [CFString: Any] = [
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: service,
            kSecAttrAccount: provider.rawValue
        ]
        SecItemDelete(query as CFDictionary)
    }

    func hasAnyKey() -> Bool {
        LLMProvider.allCases.contains { getKey(for: $0) != nil }
    }
}
