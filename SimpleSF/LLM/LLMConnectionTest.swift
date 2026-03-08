import Foundation

enum LLMConnectionTest {
    static func test(provider: LLMProvider, key: String) async throws {
        // Use a minimal models/list endpoint to verify key validity
        let urlString: String
        var headers: [String: String] = ["Authorization": "Bearer \(key)"]

        switch provider {
        case .openrouter, .openai, .kimi, .minimax, .qwen, .glm:
            urlString = "\(provider.baseURL)/models"
        case .anthropic:
            urlString = "\(provider.baseURL)/models"
            headers = ["x-api-key": key, "anthropic-version": "2023-06-01"]
        case .gemini:
            urlString = "\(provider.baseURL)/models?key=\(key)"
            headers = [:]
        }

        guard let url = URL(string: urlString) else {
            throw URLError(.badURL)
        }

        var req = URLRequest(url: url, timeoutInterval: 10)
        req.httpMethod = "GET"
        for (k, v) in headers { req.setValue(v, forHTTPHeaderField: k) }

        let (_, response) = try await URLSession.shared.data(for: req)
        guard let http = response as? HTTPURLResponse, http.statusCode < 300 else {
            let code = (response as? HTTPURLResponse)?.statusCode ?? 0
            throw NSError(domain: "LLMTest", code: code,
                          userInfo: [NSLocalizedDescriptionKey: "HTTP \(code) — invalid key or quota exceeded"])
        }
    }
}
