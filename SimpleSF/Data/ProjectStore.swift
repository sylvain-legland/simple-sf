import Foundation

struct Project: Codable, Identifiable, Hashable {
    var id: String = UUID().uuidString
    var name: String
    var description: String = ""
    var tech: String = ""
    var status: ProjectStatus = .idea
    var progress: Double = 0.0
    var createdAt: String = ISO8601DateFormatter().string(from: Date())
    var updatedAt: String = ISO8601DateFormatter().string(from: Date())
    var path: String?
    var gitURL: String?
    var missionId: String?

    static func == (lhs: Project, rhs: Project) -> Bool { lhs.id == rhs.id }
    func hash(into hasher: inout Hasher) { hasher.combine(id) }
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
        guard FileManager.default.fileExists(atPath: storeURL.path) else { return }
        do {
            let data = try Data(contentsOf: storeURL)
            projects = try JSONDecoder().decode([Project].self, from: data)
        } catch {
            print("[ProjectStore] decode error: \(error) — starting fresh")
            projects = []
        }
    }

    func save() {
        let enc = JSONEncoder()
        enc.outputFormatting = .prettyPrinted
        guard let data = try? enc.encode(projects) else { return }
        try? data.write(to: storeURL, options: .atomic)
    }

    func add(_ project: Project) {
        projects.insert(project, at: 0)
        save()
    }

    func update(_ project: Project) {
        if let idx = projects.firstIndex(where: { $0.id == project.id }) {
            var p = project
            p.updatedAt = ISO8601DateFormatter().string(from: Date())
            projects[idx] = p
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
            projects[idx].updatedAt = ISO8601DateFormatter().string(from: Date())
            save()
        }
    }

    func setMissionId(_ projectId: String, missionId: String) {
        if let idx = projects.firstIndex(where: { $0.id == projectId }) {
            projects[idx].missionId = missionId
            save()
        }
    }
}
