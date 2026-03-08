import SwiftUI
import WebKit

// Advanced mode: renders SF web UI in a native WKWebView panel
// (only used for complex views like Portfolio, PI, ART, etc.)
struct WebSFView: View {
    let path: String
    @EnvironmentObject var launcher: PlatformLauncher

    var body: some View {
        WebViewRepresentable(
            url: URL(string: "http://127.0.0.1:\(launcher.port)\(path)")!
        )
        .ignoresSafeArea()
    }
}

struct WebViewRepresentable: NSViewRepresentable {
    let url: URL

    func makeNSView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.websiteDataStore = .nonPersistent()
        let webView = WKWebView(frame: .zero, configuration: config)
        webView.customUserAgent = "SimpleSF/1.0 macOS"
        // Auto-login via cookie injection once server is ready
        Task { @MainActor in
            if let cookieHeader = await autoLogin() {
                let parts = cookieHeader.components(separatedBy: ";")
                for part in parts {
                    let kv = part.trimmingCharacters(in: .whitespaces).components(separatedBy: "=")
                    guard kv.count == 2 else { continue }
                    let cookie = HTTPCookie(properties: [
                        .name: kv[0], .value: kv[1],
                        .domain: "127.0.0.1", .path: "/"
                    ])
                    if let cookie { await webView.configuration.websiteDataStore.httpCookieStore.setCookie(cookie) }
                }
                webView.load(URLRequest(url: url))
            }
        }
        return webView
    }

    func updateNSView(_ webView: WKWebView, context: Context) {
        if webView.url?.absoluteString != url.absoluteString {
            webView.load(URLRequest(url: url))
        }
    }

    private func autoLogin() async -> String? {
        guard let loginURL = URL(string: url.absoluteString.replacingOccurrences(of: url.path, with: "/api/auth/demo")) else { return nil }
        var req = URLRequest(url: loginURL)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.httpBody = try? JSONSerialization.data(withJSONObject: ["password": "demo2026"])
        guard let (_, response) = try? await URLSession.shared.data(for: req),
              let http = response as? HTTPURLResponse,
              let cookie = http.allHeaderFields["Set-Cookie"] as? String else { return nil }
        return cookie
    }
}
