import SwiftUI

struct SettingsRootView: View {
    @EnvironmentObject var appState: AppState
    @State private var selectedTab = "providers"

    var body: some View {
        TabView(selection: $selectedTab) {
            ProvidersSettingsView()
                .tabItem { Label("Providers", systemImage: "key.fill") }
                .tag("providers")

            LanguageSettingsView()
                .tabItem { Label("Language", systemImage: "globe") }
                .tag("language")

            OutputSettingsView()
                .tabItem { Label("Output", systemImage: "arrow.up.doc") }
                .tag("output")

            AboutView()
                .tabItem { Label("About", systemImage: "info.circle") }
                .tag("about")
        }
        .frame(width: 520, height: 400)
    }
}

struct ProvidersSettingsView: View {
    @State private var keys: [LLMProvider: String] = [:]

    var body: some View {
        ScrollView {
            VStack(spacing: 10) {
                ForEach(LLMProvider.allCases) { p in
                    HStack {
                        Text(p.displayName).frame(width: 160, alignment: .leading)
                        SecureField("API key", text: Binding(
                            get: { keys[p] ?? KeychainStore.shared.getKey(for: p) ?? "" },
                            set: {
                                keys[p] = $0
                                if $0.isEmpty { KeychainStore.shared.deleteKey(for: p) }
                                else { KeychainStore.shared.setKey($0, for: p) }
                            }
                        )).textFieldStyle(.roundedBorder)
                    }
                }
            }.padding(20)
        }
    }
}

struct LanguageSettingsView: View {
    @EnvironmentObject var appState: AppState

    let languages: [(code: String, name: String)] = [
        ("af","Afrikaans"),("ar","العربية"),("bn","বাংলা"),("ca","Català"),
        ("cs","Čeština"),("da","Dansk"),("de","Deutsch"),("el","Ελληνικά"),
        ("en","English"),("es","Español"),("fi","Suomi"),("fr","Français"),
        ("he","עברית"),("hi","हिन्दी"),("hr","Hrvatski"),("hu","Magyar"),
        ("id","Indonesia"),("it","Italiano"),("ja","日本語"),("ko","한국어"),
        ("lt","Lietuvių"),("lv","Latviešu"),("ms","Melayu"),("nb","Norsk"),
        ("nl","Nederlands"),("pl","Polski"),("pt","Português"),("ro","Română"),
        ("ru","Русский"),("sk","Slovenčina"),("sl","Slovenščina"),("sr","Srpski"),
        ("sv","Svenska"),("th","ไทย"),("tl","Filipino"),("tr","Türkçe"),
        ("uk","Українська"),("ur","اردو"),("vi","Tiếng Việt"),("zh","中文")
    ]

    var body: some View {
        List(languages, id: \.code) { lang in
            HStack {
                Text("\(lang.name) (\(lang.code))")
                Spacer()
                if appState.selectedLang == lang.code {
                    Image(systemName: "checkmark").foregroundStyle(.purple)
                }
            }
            .contentShape(Rectangle())
            .onTapGesture { appState.setLanguage(lang.code) }
        }
        .frame(height: 340)
    }
}

struct OutputSettingsView: View {
    @AppStorage("sf_git_github_token") private var githubToken = ""
    @AppStorage("sf_git_gitlab_token") private var gitlabToken = ""
    @AppStorage("sf_git_default_branch") private var defaultBranch = "main"

    var body: some View {
        Form {
            Section("GitHub") {
                SecureField("Personal Access Token", text: $githubToken)
            }
            Section("GitLab") {
                SecureField("Personal Access Token", text: $gitlabToken)
            }
            Section("Git") {
                TextField("Default branch", text: $defaultBranch)
            }
        }
        .formStyle(.grouped)
        .padding(20)
    }
}

struct AboutView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "cpu.fill")
                .resizable().scaledToFit().frame(width: 56)
                .foregroundStyle(.purple)
            Text("Simple SF").font(.title.bold())
            Text("Version 1.0.0").foregroundStyle(.secondary)
            Text("Powered by the Macaron Software Factory platform.\nAll computation runs locally — your data never leaves your machine.")
                .multilineTextAlignment(.center)
                .foregroundStyle(.secondary)
                .frame(maxWidth: 360)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
