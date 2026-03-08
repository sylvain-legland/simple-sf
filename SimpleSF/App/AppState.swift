import Foundation
import Combine

enum SFMode: String { case simple, advanced }

final class AppState: ObservableObject {
    static let shared = AppState()

    @Published var mode: SFMode = .simple
    @Published var selectedLang: String = Locale.current.language.languageCode?.identifier ?? "en"
    @Published var isOnboarded: Bool = false
    @Published var showAbout: Bool = false
    @Published var serverReady: Bool = false

    private init() {
        if let saved = UserDefaults.standard.string(forKey: "sf_mode"),
           let m = SFMode(rawValue: saved) { mode = m }
        isOnboarded = UserDefaults.standard.bool(forKey: "sf_onboarded")
        if let lang = UserDefaults.standard.string(forKey: "sf_lang") { selectedLang = lang }
    }

    func setMode(_ m: SFMode) {
        mode = m
        UserDefaults.standard.set(m.rawValue, forKey: "sf_mode")
    }

    func setLanguage(_ lang: String) {
        selectedLang = lang
        UserDefaults.standard.set(lang, forKey: "sf_lang")
    }

    func completeOnboarding() {
        isOnboarded = true
        UserDefaults.standard.set(true, forKey: "sf_onboarded")
    }
}
