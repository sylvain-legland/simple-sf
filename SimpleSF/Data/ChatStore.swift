import Foundation

struct ChatSession: Codable, Identifiable {
    var id: String = UUID().uuidString
    var title: String
    var projectId: String?
    var messages: [LLMMessage] = []
    var createdAt: Date = Date()
    var updatedAt: Date = Date()
}

@MainActor
final class ChatStore: ObservableObject {
    static let shared = ChatStore()

    @Published var sessions: [ChatSession] = []
    @Published var activeSession: ChatSession?

    private var storeURL: URL {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
            .appendingPathComponent("SimpleSF", isDirectory: true)
        return dir.appendingPathComponent("chats.json")
    }

    private init() { load() }

    func load() {
        guard let data = try? Data(contentsOf: storeURL),
              let list = try? JSONDecoder().decode([ChatSession].self, from: data)
        else { return }
        sessions = list
        activeSession = sessions.first
    }

    func save() {
        guard let data = try? JSONEncoder().encode(sessions) else { return }
        try? data.write(to: storeURL, options: .atomic)
    }

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
            sessions[idx].updatedAt = Date()
            if sessions[idx].messages.count == 1 {
                sessions[idx].title = String(msg.content.prefix(40))
            }
            if activeSession?.id == sessionId { activeSession = sessions[idx] }
            save()
        }
    }
}
