import Foundation

@MainActor
final class AppState: ObservableObject {
    static let shared = AppState()

    @Published var selectedLang: String = Locale.current.language.languageCode?.identifier ?? "en"
    @Published var hasCompletedSetup: Bool
    @Published var preferredLocalProvider: String // "mlx" or "ollama"

    private init() {
        hasCompletedSetup = UserDefaults.standard.bool(forKey: "sf_setup_done")
        preferredLocalProvider = UserDefaults.standard.string(forKey: "sf_local_provider") ?? "mlx"
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

    func completeSetup() {
        hasCompletedSetup = true
        UserDefaults.standard.set(true, forKey: "sf_setup_done")
    }
}
