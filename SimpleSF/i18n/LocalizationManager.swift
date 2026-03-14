import Foundation

// Ref: FT-SSF-015
// Localization manager: loads JSON locale files, resolves strings,
// supports pluralization and interpolation.
// Detection order: UserDefaults override > System locale > English fallback.

@MainActor
final class L10n: ObservableObject, Sendable {
    static let shared = L10n()

    /// All 40 supported language codes
    static let supportedLanguages: [String] = [
        // LTR (34)
        "en", "fr", "es", "pt", "de", "it", "nl", "pl", "ro", "cs",
        "sk", "hu", "hr", "bg", "uk", "ru", "el", "tr", "vi", "th",
        "ko", "ja", "zh", "id", "ms", "tl", "hi", "bn", "sw",
        "am", "ha", "yo", "ig", "sv",
        // RTL (6)
        "ar", "he", "fa", "ur", "ps", "ku"
    ]

    /// RTL language codes
    static let rtlLanguages: Set<String> = ["ar", "he", "fa", "ur", "ps", "ku"]

    /// Language display names (in their native script)
    static let languageNames: [String: String] = [
        "en": "English", "fr": "Français", "es": "Español", "pt": "Português",
        "de": "Deutsch", "it": "Italiano", "nl": "Nederlands", "pl": "Polski",
        "ro": "Română", "cs": "Čeština", "sk": "Slovenčina", "hu": "Magyar",
        "hr": "Hrvatski", "bg": "Български", "uk": "Українська", "ru": "Русский",
        "el": "Ελληνικά", "tr": "Türkçe", "vi": "Tiếng Việt", "th": "ไทย",
        "ko": "한국어", "ja": "日本語", "zh": "中文", "id": "Bahasa Indonesia",
        "ms": "Bahasa Melayu", "tl": "Tagalog", "hi": "हिन्दी", "bn": "বাংলা",
        "sw": "Kiswahili", "am": "አማርኛ", "ha": "Hausa", "yo": "Yorùbá",
        "ig": "Igbo", "sv": "Svenska",
        "ar": "العربية", "he": "עברית", "fa": "فارسی", "ur": "اردو",
        "ps": "پښتو", "ku": "کوردی"
    ]

    @Published private(set) var currentLocale: String
    @Published private(set) var isRTL: Bool

    private var strings: [String: Any] = [:]
    private var fallbackStrings: [String: Any] = [:]

    private init() {
        // Detection order: UserDefaults > System locale > English
        let override = UserDefaults.standard.string(forKey: "sf_lang")
        let systemLang = Locale.current.language.languageCode?.identifier ?? "en"
        let resolved = override ?? (Self.supportedLanguages.contains(systemLang) ? systemLang : "en")

        currentLocale = resolved
        isRTL = Self.rtlLanguages.contains(resolved)

        // Load English as fallback, then current locale
        fallbackStrings = Self.loadLocaleFile("en")
        strings = resolved == "en" ? fallbackStrings : Self.loadLocaleFile(resolved)
    }

    // MARK: - Public API

    /// Translate a string key
    func t(_ key: StringKey) -> String {
        resolve(key.rawValue)
    }

    /// Translate a raw string key (for dynamic keys)
    func t(_ key: String) -> String {
        resolve(key)
    }

    /// Translate with interpolation: t(.setupHardware, "M3 Max", "64")
    func t(_ key: StringKey, _ args: any CVarArg...) -> String {
        let template = resolve(key.rawValue)
        return String(format: template, arguments: args)
    }

    /// Pluralization: plural(.pluralAgents, count: 5) → "5 agents"
    func plural(_ key: StringKey, count: Int) -> String {
        let rawKey = key.rawValue
        let form = pluralForm(for: count, locale: currentLocale)
        let pluralKey = "\(rawKey).\(form)"

        if let value = lookupValue(pluralKey) as? String {
            return String(format: value, count)
        }
        // Fallback to "other" form
        if let value = lookupValue("\(rawKey).other") as? String {
            return String(format: value, count)
        }
        return "\(count)"
    }

    /// Switch locale at runtime
    func setLocale(_ lang: String) {
        guard Self.supportedLanguages.contains(lang) else { return }
        currentLocale = lang
        isRTL = Self.rtlLanguages.contains(lang)
        strings = lang == "en" ? fallbackStrings : Self.loadLocaleFile(lang)
        UserDefaults.standard.set(lang, forKey: "sf_lang")
        AppState.shared.setLanguage(lang)
    }

    /// Check if current locale is RTL
    var layoutDirection: LayoutDirection {
        isRTL ? .rightToLeft : .leftToRight
    }

    // MARK: - Private

    private func resolve(_ key: String) -> String {
        if let value = lookupValue(key, in: strings) as? String {
            return value
        }
        if let value = lookupValue(key, in: fallbackStrings) as? String {
            return value
        }
        return key
    }

    private func lookupValue(_ key: String) -> Any? {
        lookupValue(key, in: strings) ?? lookupValue(key, in: fallbackStrings)
    }

    private func lookupValue(_ key: String, in dict: [String: Any]) -> Any? {
        let parts = key.split(separator: ".").map(String.init)
        var current: Any = dict
        for part in parts {
            guard let d = current as? [String: Any], let next = d[part] else {
                return nil
            }
            current = next
        }
        return current is String ? current : nil
    }

    /// CLDR plural rules (simplified for most common languages)
    private func pluralForm(for count: Int, locale: String) -> String {
        switch locale {
        case "fr", "pt", "hi", "bn":
            // French-style: 0 and 1 are singular
            return count <= 1 ? "one" : "other"
        case "ar":
            // Arabic: zero, one, two, few, many, other
            switch count {
            case 0: return "zero"
            case 1: return "one"
            case 2: return "two"
            case 3...10: return "few"
            case 11...99: return "many"
            default: return "other"
            }
        case "pl", "cs", "sk", "hr", "ru", "uk", "bg":
            // Slavic: one, few, many, other
            let mod10 = count % 10
            let mod100 = count % 100
            if count == 1 { return "one" }
            if mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14) { return "few" }
            return "many"
        case "ja", "ko", "zh", "th", "vi", "id", "ms", "tr":
            // No plural forms
            return "other"
        default:
            // English-style: 1 is singular, rest is plural
            return count == 1 ? "one" : "other"
        }
    }

    // MARK: - File loading

    private static func loadLocaleFile(_ lang: String) -> [String: Any] {
        // Search order: bundle resource, then filesystem
        let candidates = localeFileCandidates(lang)
        for url in candidates {
            if let data = try? Data(contentsOf: url),
               let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
                return json
            }
        }
        return [:]
    }

    private static func localeFileCandidates(_ lang: String) -> [URL] {
        var urls: [URL] = []
        let bundleName = "SimpleSF_SimpleSF"

        // 1. Main bundle resource
        if let url = Bundle.main.url(forResource: lang, withExtension: "json", subdirectory: "Locales") {
            urls.append(url)
        }
        // 2. Named bundle (SPM)
        if let path = Bundle.main.url(forResource: bundleName, withExtension: "bundle"),
           let bundle = Bundle(url: path),
           let url = bundle.url(forResource: lang, withExtension: "json", subdirectory: "Locales") {
            urls.append(url)
        }
        // 3. Process bundle
        if let url = Bundle(for: BundleAnchor.self).url(forResource: lang, withExtension: "json", subdirectory: "Locales") {
            urls.append(url)
        }
        // 4. Relative to executable (dev)
        let execDir = Bundle.main.bundleURL
        let devPaths = [
            execDir.appendingPathComponent("SimpleSF_SimpleSF.bundle/Locales/\(lang).json"),
            execDir.deletingLastPathComponent().appendingPathComponent("SimpleSF_SimpleSF.bundle/Locales/\(lang).json"),
        ]
        urls.append(contentsOf: devPaths)

        return urls
    }
}

// Bundle anchor for resource lookup
private final class BundleAnchor {}

// MARK: - Convenience global function

/// Shorthand: t(.navJarvis) anywhere in the app
@MainActor
func t(_ key: StringKey) -> String {
    L10n.shared.t(key)
}

/// Shorthand with interpolation: t(.setupHardware, "M3", "64")
@MainActor
func t(_ key: StringKey, _ args: any CVarArg...) -> String {
    let template = L10n.shared.t(key)
    return String(format: template, arguments: args)
}

/// Shorthand plural: plural(.pluralAgents, count: 5)
@MainActor
func plural(_ key: StringKey, count: Int) -> String {
    L10n.shared.plural(key, count: count)
}
