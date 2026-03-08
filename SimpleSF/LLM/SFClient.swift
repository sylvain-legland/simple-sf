import Foundation

final class SFClient {
    static let shared = SFClient()
    private var session: URLSession = .shared
    private var baseURL: String { "http://127.0.0.1:\(PlatformLauncher.shared.port)" }
    private var authCookie: String? = nil

    private init() {}

    // MARK: - Auth

    func login() async throws {
        let body = ["password": "demo2026"]
        let data = try JSONSerialization.data(withJSONObject: body)
        var req = request(path: "/api/auth/demo", method: "POST")
        req.httpBody = data
        let (_, response) = try await session.data(for: req)
        if let http = response as? HTTPURLResponse,
           let cookie = http.allHeaderFields["Set-Cookie"] as? String {
            authCookie = cookie.components(separatedBy: ";").first
        }
    }

    // MARK: - GET

    func get<T: Decodable>(_ path: String) async throws -> T {
        let req = request(path: path)
        let (data, _) = try await session.data(for: req)
        return try JSONDecoder().decode(T.self, from: data)
    }

    func getRaw(_ path: String) async throws -> Any {
        let req = request(path: path)
        let (data, _) = try await session.data(for: req)
        return try JSONSerialization.jsonObject(with: data)
    }

    // MARK: - POST

    func post<T: Decodable>(_ path: String, body: Encodable) async throws -> T {
        var req = request(path: path, method: "POST")
        req.httpBody = try JSONEncoder().encode(body)
        let (data, _) = try await session.data(for: req)
        return try JSONDecoder().decode(T.self, from: data)
    }

    func postRaw(_ path: String, body: [String: Any]) async throws -> Any {
        var req = request(path: path, method: "POST")
        req.httpBody = try JSONSerialization.data(withJSONObject: body)
        let (data, _) = try await session.data(for: req)
        return try JSONSerialization.jsonObject(with: data)
    }

    // MARK: - SSE Streaming

    func stream(_ path: String, body: [String: Any]? = nil) -> AsyncThrowingStream<String, Error> {
        AsyncThrowingStream { continuation in
            Task {
                var req = request(path: path, method: body != nil ? "POST" : "GET")
                req.timeoutInterval = 300
                if let body {
                    req.httpBody = try? JSONSerialization.data(withJSONObject: body)
                }
                let (bytes, _) = try await URLSession.shared.bytes(for: req)
                var buffer = ""
                for try await byte in bytes {
                    let char = String(bytes: [byte], encoding: .utf8) ?? ""
                    buffer += char
                    if buffer.hasSuffix("\n") {
                        let line = buffer.trimmingCharacters(in: .whitespacesAndNewlines)
                        buffer = ""
                        if line.hasPrefix("data: ") {
                            let payload = String(line.dropFirst(6))
                            if payload != "[DONE]" { continuation.yield(payload) }
                        }
                    }
                }
                continuation.finish()
            }
        }
    }

    // MARK: - Helpers

    private func request(path: String, method: String = "GET") -> URLRequest {
        var req = URLRequest(url: URL(string: "\(baseURL)\(path)")!)
        req.httpMethod = method
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.setValue("application/json", forHTTPHeaderField: "Accept")
        if let cookie = authCookie { req.setValue(cookie, forHTTPHeaderField: "Cookie") }
        return req
    }
}
