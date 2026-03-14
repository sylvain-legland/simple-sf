import SwiftUI

// MARK: - SF Platform Catalog
// Loads 192 agents, 19 patterns, 42 workflows, 1286 skills from bundled JSON.
// Source of truth: platform/agents/store.py → exported to SFData/*.json

// MARK: - Models

// Ref: FT-SSF-010
struct SFAgent: Codable, Identifiable {
    let id: String
    let name: String
    let role: String
    let description: String?
    let icon: String?
    let color: String?
    let avatar: String?       // initials e.g. "GM"
    let tagline: String?
    let persona: String?
    let motivation: String?
    let hierarchyRank: Int?
    let projectId: String?

    // Decoded from JSON string fields
    let skillsJson: String?
    let toolsJson: String?
    let tagsJson: String?
    let permissionsJson: String?

    enum CodingKeys: String, CodingKey {
        case id, name, role, description, icon, color, avatar, tagline, persona, motivation
        case hierarchyRank = "hierarchy_rank"
        case projectId = "project_id"
        case skillsJson = "skills_json"
        case toolsJson = "tools_json"
        case tagsJson = "tags_json"
        case permissionsJson = "permissions_json"
    }

    var swiftColor: Color {
        guard let hex = color, !hex.isEmpty else { return .purple }
        let clean = hex.hasPrefix("#") ? String(hex.dropFirst()) : hex
        guard let val = UInt(clean, radix: 16) else { return .purple }
        return Color(hex: val)
    }

    var sfIcon: String {
        switch icon {
        case "brain":        return "brain.head.profile"
        case "code":         return "laptopcomputer"
        case "eye":          return "eye"
        case "shield":       return "lock.shield"
        case "building":     return "building.2"
        case "cloud":        return "cloud"
        case "server":       return "server.rack"
        case "terminal":     return "terminal"
        case "search":       return "magnifyingglass"
        case "database":     return "cylinder"
        case "chart":        return "chart.bar"
        case "paintbrush":   return "paintbrush"
        case "phone":        return "iphone"
        case "gear":         return "gearshape"
        case "flag":         return "flag"
        case "book":         return "book"
        case "star":         return "star"
        case "lock":         return "lock"
        case "wrench":       return "wrench"
        case "puzzle":       return "puzzlepiece"
        case "zap":          return "bolt"
        case "megaphone":    return "megaphone"
        case "heart":        return "heart"
        case "users":        return "person.3"
        case "target":       return "target"
        case "compass":      return "safari"
        case "layers":       return "square.3.layers.3d"
        case "cpu":          return "cpu"
        case "globe":        return "globe"
        case "feather":      return "leaf"
        default:             return "person.fill"
        }
    }

    var skills: [String] {
        guard let json = skillsJson, let data = json.data(using: .utf8),
              let arr = try? JSONDecoder().decode([String].self, from: data) else { return [] }
        return arr
    }

    var tools: [String] {
        guard let json = toolsJson, let data = json.data(using: .utf8),
              let arr = try? JSONDecoder().decode([String].self, from: data) else { return [] }
        return arr
    }

    var tags: [String] {
        guard let json = tagsJson, let data = json.data(using: .utf8),
              let arr = try? JSONDecoder().decode([String].self, from: data) else { return [] }
        return arr
    }

    var permissions: [String: Bool] {
        guard let json = permissionsJson, let data = json.data(using: .utf8),
              let dict = try? JSONDecoder().decode([String: Bool].self, from: data) else { return [:] }
        return dict
    }
}

struct SFPattern: Codable, Identifiable {
    let id: String
    let name: String
    let description: String?
    let type: String?
    let icon: String?

    enum CodingKeys: String, CodingKey {
        case id, name, description, type, icon
    }
}

struct SFWorkflow: Codable, Identifiable {
    let id: String
    let name: String
    let description: String?
    let icon: String?
    let phasesJson: String?

    enum CodingKeys: String, CodingKey {
        case id, name, description, icon
        case phasesJson = "phases_json"
    }
}

struct SFSkill: Codable, Identifiable {
    let id: String
    let name: String
    let description: String?
    let source: String?
    let tagsJson: String?

    enum CodingKeys: String, CodingKey {
        case id, name, description, source
        case tagsJson = "tags_json"
    }

    var tags: [String] {
        guard let json = tagsJson, let data = json.data(using: .utf8),
              let arr = try? JSONDecoder().decode([String].self, from: data) else { return [] }
        return arr
    }
}

// MARK: - Catalog Singleton

@MainActor
final class SFCatalog: ObservableObject {
    static let shared = SFCatalog()

    @Published private(set) var agents: [SFAgent] = []
    @Published private(set) var patterns: [SFPattern] = []
    @Published private(set) var workflows: [SFWorkflow] = []
    @Published private(set) var skillCount: Int = 0

    private var agentIndex: [String: SFAgent] = [:]

    private init() {
        loadAll()
    }

    // O(1) lookup by agent ID
    func agent(id: String) -> SFAgent? {
        agentIndex[id]
    }

    func agentName(_ id: String) -> String {
        agentIndex[id]?.name ?? id
    }

    func agentRole(_ id: String) -> String {
        agentIndex[id]?.role ?? ""
    }

    func agentColor(_ id: String) -> Color {
        agentIndex[id]?.swiftColor ?? SF.Colors.purple
    }

    func agentIcon(_ id: String) -> String {
        agentIndex[id]?.sfIcon ?? "person.fill"
    }

    // Filtered views
    var strategicAgents: [SFAgent] {
        agents.filter { ($0.hierarchyRank ?? 50) <= 10 }
    }

    var teamLeads: [SFAgent] {
        agents.filter { $0.role.lowercased().contains("lead") || $0.role.lowercased().contains("rte") }
    }

    var securityAgents: [SFAgent] {
        agents.filter { $0.tags.contains("security") || $0.role.lowercased().contains("security") || $0.role.lowercased().contains("secops") || $0.role.lowercased().contains("pentester") || $0.role.lowercased().contains("ciso") }
    }

    var projectAgents: [SFAgent] {
        agents.filter { $0.projectId != nil && !($0.projectId?.isEmpty ?? true) }
    }

    var builtinAgents: [SFAgent] {
        agents.filter { $0.projectId == nil || $0.projectId?.isEmpty == true }
    }

    func agentsForProject(_ projectId: String) -> [SFAgent] {
        agents.filter { $0.projectId == projectId }
    }

    // MARK: - Loading

    private func loadAll() {
        agents = loadJSON("agents") ?? []
        agentIndex = Dictionary(uniqueKeysWithValues: agents.map { ($0.id, $0) })

        patterns = loadJSON("patterns") ?? []
        workflows = loadJSON("workflows") ?? []

        // Skills are big (9MB) — just count them
        if let data = loadData("skills") {
            struct Wrapper: Decodable { let id: String }
            skillCount = (try? JSONDecoder().decode([Wrapper].self, from: data))?.count ?? 0
        }

        print("[SFCatalog] Loaded \(agents.count) agents, \(patterns.count) patterns, \(workflows.count) workflows, \(skillCount) skills")
    }

    private func loadJSON<T: Decodable>(_ name: String) -> T? {
        guard let data = loadData(name) else { return nil }
        do {
            return try JSONDecoder().decode(T.self, from: data)
        } catch {
            print("[SFCatalog] Failed to decode \(name).json: \(error)")
            return nil
        }
    }

    private func loadData(_ name: String) -> Data? {
        // 1. SPM resource bundle
        let bundleCandidates = [
            Bundle.main.resourceURL?.appendingPathComponent("SimpleSF_SimpleSF.bundle"),
            Bundle.main.bundleURL.appendingPathComponent("Contents/Resources/SimpleSF_SimpleSF.bundle"),
            Bundle.main.executableURL?.deletingLastPathComponent().appendingPathComponent("SimpleSF_SimpleSF.bundle"),
        ].compactMap { $0 }

        for bundleURL in bundleCandidates {
            if let bundle = Bundle(url: bundleURL) {
                if let url = bundle.url(forResource: name, withExtension: "json", subdirectory: "SFData") {
                    if let data = try? Data(contentsOf: url) { return data }
                }
            }
            // Direct file path
            let direct = bundleURL.appendingPathComponent("SFData/\(name).json")
            if let data = try? Data(contentsOf: direct) { return data }
        }

        // 2. Main bundle direct
        if let url = Bundle.main.url(forResource: name, withExtension: "json", subdirectory: "SFData") {
            if let data = try? Data(contentsOf: url) { return data }
        }

        print("[SFCatalog] Could not find \(name).json in any bundle")
        return nil
    }
}
