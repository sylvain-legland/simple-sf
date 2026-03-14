import Foundation
import Security

// Ref: FT-SSF-005
@MainActor
final class KeychainService: ObservableObject {
    static let shared = KeychainService()
    let service = "com.simple-sf.apikeys"

    @Published var storedProviders: Set<LLMProvider> = []
    private var hasScanned = false

    private init() {}

    /// Scan stored providers on a background thread to avoid blocking the main thread.
    /// SecItemCopyMatching can block if binary signature changed.
    func scanIfNeeded() async {
        guard !hasScanned else { return }
        hasScanned = true
        let svc = service
        let found: Set<LLMProvider> = await Task.detached {
            var result = Set<LLMProvider>()
            for provider in LLMProvider.allCases {
                let query: [String: Any] = [
                    kSecClass as String: kSecClassGenericPassword,
                    kSecAttrService as String: svc,
                    kSecAttrAccount as String: provider.rawValue,
                    kSecReturnData as String: true,
                    kSecMatchLimit as String: kSecMatchLimitOne
                ]
                var item: CFTypeRef?
                if SecItemCopyMatching(query as CFDictionary, &item) == errSecSuccess,
                   let data = item as? Data,
                   let str = String(data: data, encoding: .utf8), !str.isEmpty {
                    result.insert(provider)
                }
            }
            return result
        }.value
        storedProviders = found
    }

    func key(for provider: LLMProvider) -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: provider.rawValue,
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

    func save(key: String, for provider: LLMProvider) {
        guard let data = key.data(using: .utf8) else { return }
        delete(for: provider)
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: provider.rawValue,
            kSecValueData as String: data
        ]
        SecItemAdd(query as CFDictionary, nil)
        if !key.isEmpty { storedProviders.insert(provider) }
        else { storedProviders.remove(provider) }
    }

    func delete(for provider: LLMProvider) {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: provider.rawValue
        ]
        SecItemDelete(query as CFDictionary)
        storedProviders.remove(provider)
    }
}
