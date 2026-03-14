import Foundation
import Security

// Ref: FT-SSF-002

// MARK: - C FFI declarations (LLM configuration)

@_silgen_name("sf_configure_llm")
func _sf_configure_llm(_ provider: UnsafePointer<CChar>?, _ apiKey: UnsafePointer<CChar>?, _ baseUrl: UnsafePointer<CChar>?, _ model: UnsafePointer<CChar>?)

@_silgen_name("sf_set_yolo")
func _sf_set_yolo(_ enabled: Bool)

// MARK: - LLM & Settings Configuration

extension SFBridge {

    /// Sync YOLO mode to the Rust engine
    func syncYoloMode() {
        _sf_set_yolo(AppState.shared.yoloMode)
    }

    /// Synchronous config sync — used after user-initiated provider changes.
    /// NOTE: calls SecItemCopyMatching which may block if keychain is locked.
    func syncLLMConfig() {
        let state = AppState.shared
        if let provider = state.selectedProvider {
            let model = state.selectedModel.isEmpty ? provider.defaultModel : state.selectedModel
            switch provider {
            case .mlx:
                configureLLM(provider: "mlx", apiKey: "no-key", baseUrl: MLXService.shared.baseURL,
                             model: MLXService.shared.activeModel?.name ?? model)
                return
            case .ollama:
                if OllamaService.shared.isRunning, let m = OllamaService.shared.activeModel {
                    configureLLM(provider: "ollama", apiKey: "no-key", baseUrl: OllamaService.shared.openaiBaseURL,
                                 model: m.name)
                    return
                }
            default:
                if let apiKey = KeychainService.shared.key(for: provider) {
                    configureLLM(provider: provider.rawValue, apiKey: apiKey,
                                 baseUrl: provider.baseURL, model: model)
                    return
                }
            }
        }
        if MLXService.shared.isRunning {
            configureLLM(provider: "mlx", apiKey: "no-key", baseUrl: MLXService.shared.baseURL,
                         model: MLXService.shared.activeModel?.name ?? "mlx-local")
            return
        }
        if OllamaService.shared.isRunning, let m = OllamaService.shared.activeModel {
            configureLLM(provider: "ollama", apiKey: "no-key", baseUrl: OllamaService.shared.openaiBaseURL,
                         model: m.name)
            return
        }
        let keychain = KeychainService.shared
        guard let provider = LLMProvider.cloudProviders.first(where: { keychain.storedProviders.contains($0) }),
              let apiKey = keychain.key(for: provider) else { return }
        configureLLM(provider: provider.rawValue, apiKey: apiKey,
                     baseUrl: provider.baseURL, model: provider.defaultModel)
    }

    /// Async variant — runs keychain access on background thread.
    func syncLLMConfigAsync() async {
        let state = AppState.shared
        let keychain = KeychainService.shared

        // 1. Explicit user selection takes priority
        if let provider = state.selectedProvider {
            let model = state.selectedModel.isEmpty ? provider.defaultModel : state.selectedModel
            switch provider {
            case .mlx:
                let svc = MLXService.shared
                configureLLM(provider: "mlx", apiKey: "no-key", baseUrl: svc.baseURL,
                             model: svc.activeModel?.name ?? model)
                return
            case .ollama:
                let svc = OllamaService.shared
                if svc.isRunning, let m = svc.activeModel {
                    configureLLM(provider: "ollama", apiKey: "no-key", baseUrl: svc.openaiBaseURL,
                                 model: m.name)
                    return
                }
            default:
                if keychain.storedProviders.contains(provider) {
                    let svc = keychain.service
                    let raw = provider.rawValue
                    let apiKey: String? = await Task.detached(operation: {
                        Self.keychainLookup(service: svc, account: raw)
                    }).value
                    if let apiKey {
                        configureLLM(provider: provider.rawValue, apiKey: apiKey,
                                     baseUrl: provider.baseURL, model: model)
                        return
                    }
                }
            }
        }

        // 2. Local providers
        let preferred = state.preferredLocalProvider
        if preferred == "mlx", MLXService.shared.isRunning {
            configureLLM(provider: "mlx", apiKey: "no-key", baseUrl: MLXService.shared.baseURL,
                         model: MLXService.shared.activeModel?.name ?? "mlx-local")
            return
        }
        if preferred == "ollama", OllamaService.shared.isRunning, let m = OllamaService.shared.activeModel {
            configureLLM(provider: "ollama", apiKey: "no-key", baseUrl: OllamaService.shared.openaiBaseURL,
                         model: m.name)
            return
        }
        if MLXService.shared.isRunning {
            configureLLM(provider: "mlx", apiKey: "no-key", baseUrl: MLXService.shared.baseURL,
                         model: MLXService.shared.activeModel?.name ?? "mlx-local")
            return
        }
        if OllamaService.shared.isRunning, let m = OllamaService.shared.activeModel {
            configureLLM(provider: "ollama", apiKey: "no-key", baseUrl: OllamaService.shared.openaiBaseURL,
                         model: m.name)
            return
        }

        // 3. First cloud with key
        guard let provider = LLMProvider.cloudProviders.first(where: { keychain.storedProviders.contains($0) }) else { return }
        let svc = keychain.service
        let raw = provider.rawValue
        let apiKey: String? = await Task.detached(operation: {
            Self.keychainLookup(service: svc, account: raw)
        }).value
        guard let apiKey else { return }
        configureLLM(provider: provider.rawValue, apiKey: apiKey,
                     baseUrl: provider.baseURL, model: provider.defaultModel)
    }

    /// Thread-safe keychain lookup (no @MainActor)
    nonisolated static func keychainLookup(service: String, account: String) -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        var item: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &item) == errSecSuccess,
              let data = item as? Data,
              let str = String(data: data, encoding: .utf8), !str.isEmpty
        else { return nil }
        return str
    }

    func configureLLM(provider: String, apiKey: String, baseUrl: String, model: String) {
        provider.withCString { p in
            apiKey.withCString { k in
                baseUrl.withCString { u in
                    model.withCString { m in
                        _sf_configure_llm(p, k, u, m)
                    }
                }
            }
        }
    }
}
