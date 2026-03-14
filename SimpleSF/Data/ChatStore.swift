import Foundation

// Ref: FT-SSF-009
struct ChatSession: Codable, Identifiable {
    var id: String = UUID().uuidString
    var title: String
    var projectId: String?
    var messages: [LLMMessage] = []
    var createdAt: String = ISO8601DateFormatter().string(from: Date())
    var updatedAt: String = ISO8601DateFormatter().string(from: Date())
}

@MainActor
final class ChatStore: ObservableObject {
    static let shared = ChatStore()

    @Published var sessions: [ChatSession] = []
    @Published var activeSession: ChatSession?

    private var storeURL: URL {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
            .appendingPathComponent("SimpleSF", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appendingPathComponent("chats.json")
    }

    private init() { load() }

    func load() {
        guard FileManager.default.fileExists(atPath: storeURL.path) else { return }
        do {
            let data = try Data(contentsOf: storeURL)
            sessions = try JSONDecoder().decode([ChatSession].self, from: data)
            activeSession = sessions.first
        } catch {
            print("[ChatStore] decode error: \(error) — starting fresh")
            sessions = []
        }
    }

    func save() {
        let enc = JSONEncoder()
        enc.outputFormatting = .prettyPrinted
        guard let data = try? enc.encode(sessions) else { return }
        try? data.write(to: storeURL, options: .atomic)
    }

    @discardableResult
    func newSession(title: String = "New Chat", projectId: String? = nil) -> ChatSession {
        let s = ChatSession(title: title, projectId: projectId)
        sessions.insert(s, at: 0)
        activeSession = s
        save()
        return s
    }

    func appendMessage(_ msg: LLMMessage, to sessionId: String) {
        if let idx = sessions.firstIndex(where: { $0.id == sessionId }) {
            sessions[idx].messages.append(msg)
            sessions[idx].updatedAt = ISO8601DateFormatter().string(from: Date())
            if sessions[idx].messages.count == 1 {
                sessions[idx].title = String(msg.content.prefix(40))
            }
            if activeSession?.id == sessionId { activeSession = sessions[idx] }
            save()
        }
    }
}
