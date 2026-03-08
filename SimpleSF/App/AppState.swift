import Foundation

@MainActor
final class AppState: ObservableObject {
    static let shared = AppState()

    @Published var selectedLang: String = Locale.current.language.languageCode?.identifier ?? "en"
    @Published var hasCompletedSetup: Bool
    @Published var preferredLocalProvider: String // "mlx" or "ollama"

    /// Explicitly selected provider (nil = auto-detect)
    @Published var selectedProvider: LLMProvider?
    /// Model override for the selected provider (empty = use default)
    @Published var selectedModel: String

    private init() {
        hasCompletedSetup = UserDefaults.standard.bool(forKey: "sf_setup_done")
        preferredLocalProvider = UserDefaults.standard.string(forKey: "sf_local_provider") ?? "mlx"
        if let raw = UserDefaults.standard.string(forKey: "sf_selected_provider") {
            selectedProvider = LLMProvider(rawValue: raw)
        } else {
            selectedProvider = nil
        }
        selectedModel = UserDefaults.standard.string(forKey: "sf_selected_model") ?? ""
        if let lang = UserDefaults.standard.string(forKey: "sf_lang") { selectedLang = lang }
    }

    func setLanguage(_ lang: String) {
        selectedLang = lang
        UserDefaults.standard.set(lang, forKey: "sf_lang")
    }

    func setPreferredProvider(_ provider: String) {
        preferredLocalProvider = provider
        UserDefaults.standard.set(provider, forKey: "sf_local_provider")
    }

    /// Set the active provider+model explicitly
    func setActiveProvider(_ provider: LLMProvider, model: String? = nil) {
        selectedProvider = provider
        selectedModel = model ?? provider.defaultModel
        UserDefaults.standard.set(provider.rawValue, forKey: "sf_selected_provider")
        UserDefaults.standard.set(selectedModel, forKey: "sf_selected_model")
        if provider.isLocal { setPreferredProvider(provider.rawValue) }
        SFBridge.shared.syncLLMConfig()
    }

    /// Clear explicit selection (back to auto-detect)
    func clearActiveProvider() {
        selectedProvider = nil
        selectedModel = ""
        UserDefaults.standard.removeObject(forKey: "sf_selected_provider")
        UserDefaults.standard.removeObject(forKey: "sf_selected_model")
        SFBridge.shared.syncLLMConfig()
    }

    func completeSetup() {
        hasCompletedSetup = true
        UserDefaults.standard.set(true, forKey: "sf_setup_done")
    }
}
