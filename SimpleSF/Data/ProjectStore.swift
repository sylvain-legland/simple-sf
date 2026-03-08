import Foundation

struct Project: Codable, Identifiable {
    var id: String = UUID().uuidString
    var name: String
    var description: String = ""
    var tech: String = ""
    var status: ProjectStatus = .idea
    var progress: Double = 0.0
    var createdAt: Date = Date()
    var updatedAt: Date = Date()
    var path: String?          // local workspace path
    var gitURL: String?        // remote git URL
    var chatHistory: [LLMMessage] = []
}

enum ProjectStatus: String, Codable, CaseIterable {
    case idea, planning, active, paused, done

    var displayName: String {
        switch self {
        case .idea:     return "Idea"
        case .planning: return "Planning"
        case .active:   return "Active"
        case .paused:   return "Paused"
        case .done:     return "Done"
        }
    }

    var color: String {
        switch self {
        case .idea:     return "#6366f1"
        case .planning: return "#f59e0b"
        case .active:   return "#22c55e"
        case .paused:   return "#94a3b8"
        case .done:     return "#a855f7"
        }
    }
}

@MainActor
final class ProjectStore: ObservableObject {
    static let shared = ProjectStore()

    @Published var projects: [Project] = []

    private var storeURL: URL {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
            .appendingPathComponent("SimpleSF", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appendingPathComponent("projects.json")
    }

    private init() { load() }

    func load() {
        guard let data = try? Data(contentsOf: storeURL),
              let list = try? JSONDecoder().decode([Project].self, from: data)
        else { return }
        projects = list
    }

    func save() {
        guard let data = try? JSONEncoder().encode(projects) else { return }
        try? data.write(to: storeURL, options: .atomic)
    }

    func add(_ project: Project) {
        projects.insert(project, at: 0)
        save()
    }

    func update(_ project: Project) {
        if let idx = projects.firstIndex(where: { $0.id == project.id }) {
            projects[idx] = project
            projects[idx].updatedAt = Date()
            save()
        }
    }

    func delete(_ id: String) {
        projects.removeAll { $0.id == id }
        save()
    }

    func setStatus(_ id: String, status: ProjectStatus) {
        if let idx = projects.firstIndex(where: { $0.id == id }) {
            projects[idx].status = status
            projects[idx].updatedAt = Date()
            save()
        }
    }
}
