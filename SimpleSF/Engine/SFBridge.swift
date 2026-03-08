import Foundation

// MARK: - C FFI declarations (mirrors sf_engine.h)

// We declare the C functions directly since SPM doesn't easily support bridging headers.
// These match the extern "C" functions in SFEngine/src/ffi.rs.

typealias SFEventCallback = @convention(c) (UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafePointer<CChar>?) -> Void

@_silgen_name("sf_init")
func _sf_init(_ dbPath: UnsafePointer<CChar>?)

@_silgen_name("sf_set_callback")
func _sf_set_callback(_ cb: SFEventCallback)

@_silgen_name("sf_configure_llm")
func _sf_configure_llm(_ provider: UnsafePointer<CChar>?, _ apiKey: UnsafePointer<CChar>?, _ baseUrl: UnsafePointer<CChar>?, _ model: UnsafePointer<CChar>?)

@_silgen_name("sf_create_project")
func _sf_create_project(_ name: UnsafePointer<CChar>?, _ desc: UnsafePointer<CChar>?, _ tech: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("sf_list_projects")
func _sf_list_projects() -> UnsafeMutablePointer<CChar>?

@_silgen_name("sf_delete_project")
func _sf_delete_project(_ id: UnsafePointer<CChar>?)

@_silgen_name("sf_start_mission")
func _sf_start_mission(_ projectId: UnsafePointer<CChar>?, _ brief: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("sf_mission_status")
func _sf_mission_status(_ missionId: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("sf_list_agents")
func _sf_list_agents() -> UnsafeMutablePointer<CChar>?

@_silgen_name("sf_free_string")
func _sf_free_string(_ s: UnsafeMutablePointer<CChar>?)

// MARK: - Swift-friendly wrapper

@MainActor
final class SFBridge: ObservableObject {
    static let shared = SFBridge()

    @Published var events: [AgentEvent] = []
    @Published var isRunning = false
    @Published var currentMissionId: String?
    @Published var engineReady = false

    struct AgentEvent: Identifiable {
        let id = UUID()
        let agentId: String
        let eventType: String
        let data: String
        let timestamp = Date()
    }

    private init() {}

    func initialize() {
        let appSupport = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
        let sfDir = appSupport.appendingPathComponent("SimpleSF")
        try? FileManager.default.createDirectory(at: sfDir, withIntermediateDirectories: true)
        let dbPath = sfDir.appendingPathComponent("sf_engine.db").path

        dbPath.withCString { ptr in
            _sf_init(ptr)
        }

        // Set callback (routes through a global C function)
        _sf_set_callback(sfEventHandler)

        engineReady = true
    }

    /// Pass LLM config from macOS Keychain to the Rust engine
    func syncLLMConfig() {
        let keychain = KeychainService.shared
        // Find first provider with a key (same logic as LLMService.activeProvider)
        guard let provider = LLMProvider.allCases.first(where: { keychain.key(for: $0) != nil }),
              let apiKey = keychain.key(for: provider) else { return }
        configureLLM(
            provider: provider.rawValue,
            apiKey: apiKey,
            baseUrl: provider.baseURL,
            model: provider.defaultModel
        )
    }

    func configureLLM(provider: String, apiKey: String, baseUrl: String, model: String) {
        provider.withCString { p in
            apiKey.withCString { k in
                baseUrl.withCString { u in
                    model.withCString { m in
                        _sf_configure_llm(p, k, u, m)
                    }
                }
            }
        }
    }

    func createProject(name: String, description: String, tech: String) -> String? {
        var result: String?
        name.withCString { n in
            description.withCString { d in
                tech.withCString { t in
                    if let ptr = _sf_create_project(n, d, t) {
                        result = String(cString: ptr)
                        _sf_free_string(ptr)
                    }
                }
            }
        }
        return result
    }

    struct SFProject: Codable, Identifiable, Hashable {
        let id: String
        let name: String
        let description: String
        let tech: String
        let status: String
        let created_at: String
    }

    func listProjects() -> [SFProject] {
        guard let ptr = _sf_list_projects() else { return [] }
        let json = String(cString: ptr)
        _sf_free_string(ptr)
        guard let data = json.data(using: .utf8) else { return [] }
        return (try? JSONDecoder().decode([SFProject].self, from: data)) ?? []
    }

    func deleteProject(id: String) {
        id.withCString { ptr in
            _sf_delete_project(ptr)
        }
    }

    func startMission(projectId: String, brief: String) -> String? {
        events.removeAll()
        isRunning = true
        var result: String?
        projectId.withCString { p in
            brief.withCString { b in
                if let ptr = _sf_start_mission(p, b) {
                    result = String(cString: ptr)
                    _sf_free_string(ptr)
                }
            }
        }
        currentMissionId = result
        return result
    }

    struct MissionStatus: Codable {
        let mission: MissionInfo?
        let phases: [PhaseInfo]
        let messages: [MessageInfo]
    }
    struct MissionInfo: Codable {
        let id: String
        let project_id: String
        let brief: String
        let status: String
        let created_at: String
    }
    struct PhaseInfo: Codable, Identifiable {
        let id: String
        let phase_name: String
        let pattern: String
        let status: String
        let agent_ids: String
        let output: String?
        let started_at: String?
        let completed_at: String?
    }
    struct MessageInfo: Codable, Identifiable {
        var id: String { "\(agent_name)-\(created_at)" }
        let agent_name: String
        let role: String
        let content: String
        let tool_calls: String?
        let created_at: String
    }

    func missionStatus() -> MissionStatus? {
        guard let mid = currentMissionId else { return nil }
        return mid.withCString { ptr -> MissionStatus? in
            guard let result = _sf_mission_status(ptr) else { return nil }
            let json = String(cString: result)
            _sf_free_string(result)
            guard let data = json.data(using: .utf8) else { return nil }
            return try? JSONDecoder().decode(MissionStatus.self, from: data)
        }
    }

    struct SFAgent: Codable, Identifiable {
        let id: String
        let name: String
        let role: String
        let persona: String
    }

    func listAgents() -> [SFAgent] {
        guard let ptr = _sf_list_agents() else { return [] }
        let json = String(cString: ptr)
        _sf_free_string(ptr)
        guard let data = json.data(using: .utf8) else { return [] }
        return (try? JSONDecoder().decode([SFAgent].self, from: data)) ?? []
    }

    // Called from the global C callback
    nonisolated func handleEvent(agentId: String, eventType: String, data: String) {
        Task { @MainActor in
            let event = AgentEvent(agentId: agentId, eventType: eventType, data: data)
            self.events.append(event)
            if eventType == "mission_complete" {
                self.isRunning = false
            }
        }
    }
}

// Global C callback function — routes to SFBridge singleton
private func sfEventHandler(agentId: UnsafePointer<CChar>?, eventType: UnsafePointer<CChar>?, data: UnsafePointer<CChar>?) {
    let a = agentId.map { String(cString: $0) } ?? ""
    let t = eventType.map { String(cString: $0) } ?? ""
    let d = data.map { String(cString: $0) } ?? ""
    SFBridge.shared.handleEvent(agentId: a, eventType: t, data: d)
}
