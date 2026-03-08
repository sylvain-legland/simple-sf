import Foundation
import Security

@MainActor
final class KeychainService: ObservableObject {
    static let shared = KeychainService()
    private let service = "com.simple-sf.apikeys"

    @Published var storedProviders: Set<LLMProvider> = []

    private init() {
        storedProviders = Set(LLMProvider.allCases.filter { key(for: $0) != nil })
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
