import Foundation

@MainActor
final class AppState: ObservableObject {
    static let shared = AppState()

    @Published var selectedLang: String = Locale.current.language.languageCode?.identifier ?? "en"

    private init() {
        if let lang = UserDefaults.standard.string(forKey: "sf_lang") { selectedLang = lang }
    }

    func setLanguage(_ lang: String) {
        selectedLang = lang
        UserDefaults.standard.set(lang, forKey: "sf_lang")
    }
}
