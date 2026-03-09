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

    /// JSON file for LLM settings — survives codesign (unlike UserDefaults)
    private var llmSettingsURL: URL {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
            .appendingPathComponent("SimpleSF", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appendingPathComponent("llm_settings.json")
    }

    private struct LLMSettings: Codable {
        var provider: String?
        var model: String?
        var localProvider: String?
        var lang: String?
        var setupDone: Bool?
    }

    private init() {
        // Load from JSON first (survives codesign), then fallback to UserDefaults
        let settings = Self.loadLLMSettings()

        hasCompletedSetup = settings?.setupDone ?? UserDefaults.standard.bool(forKey: "sf_setup_done")
        preferredLocalProvider = settings?.localProvider ?? UserDefaults.standard.string(forKey: "sf_local_provider") ?? "mlx"

        if let raw = settings?.provider ?? UserDefaults.standard.string(forKey: "sf_selected_provider") {
            selectedProvider = LLMProvider(rawValue: raw)
        } else {
            selectedProvider = nil
        }
        selectedModel = settings?.model ?? UserDefaults.standard.string(forKey: "sf_selected_model") ?? ""
        if let lang = settings?.lang ?? UserDefaults.standard.string(forKey: "sf_lang") { selectedLang = lang }
    }

    private static func loadLLMSettings() -> LLMSettings? {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
            .appendingPathComponent("SimpleSF", isDirectory: true)
        let url = dir.appendingPathComponent("llm_settings.json")
        guard FileManager.default.fileExists(atPath: url.path),
              let data = try? Data(contentsOf: url),
              let settings = try? JSONDecoder().decode(LLMSettings.self, from: data) else { return nil }
        return settings
    }

    private func saveLLMSettings() {
        let settings = LLMSettings(
            provider: selectedProvider?.rawValue,
            model: selectedModel.isEmpty ? nil : selectedModel,
            localProvider: preferredLocalProvider,
            lang: selectedLang,
            setupDone: hasCompletedSetup
        )
        guard let data = try? JSONEncoder().encode(settings) else { return }
        try? data.write(to: llmSettingsURL, options: .atomic)
    }

    func setLanguage(_ lang: String) {
        selectedLang = lang
        UserDefaults.standard.set(lang, forKey: "sf_lang")
        saveLLMSettings()
    }

    func setPreferredProvider(_ provider: String) {
        preferredLocalProvider = provider
        UserDefaults.standard.set(provider, forKey: "sf_local_provider")
        saveLLMSettings()
    }

    /// Set the active provider+model explicitly
    func setActiveProvider(_ provider: LLMProvider, model: String? = nil) {
        selectedProvider = provider
        selectedModel = model ?? provider.defaultModel
        UserDefaults.standard.set(provider.rawValue, forKey: "sf_selected_provider")
        UserDefaults.standard.set(selectedModel, forKey: "sf_selected_model")
        if provider.isLocal { setPreferredProvider(provider.rawValue) }
        saveLLMSettings()
        SFBridge.shared.syncLLMConfig()
    }

    /// Clear explicit selection (back to auto-detect)
    func clearActiveProvider() {
        selectedProvider = nil
        selectedModel = ""
        UserDefaults.standard.removeObject(forKey: "sf_selected_provider")
        UserDefaults.standard.removeObject(forKey: "sf_selected_model")
        saveLLMSettings()
        SFBridge.shared.syncLLMConfig()
    }

    func completeSetup() {
        hasCompletedSetup = true
        UserDefaults.standard.set(true, forKey: "sf_setup_done")
        saveLLMSettings()
    }
}
