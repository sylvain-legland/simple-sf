import Foundation
import Security

// MARK: - C FFI declarations (mirrors sf_engine.h)

// We declare the C functions directly since SPM doesn't easily support bridging headers.
// These match the extern "C" functions in SFEngine/src/ffi.rs.

typealias SFEventCallback = @convention(c) (UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafePointer<CChar>?) -> Void

@_silgen_name("sf_init")
func _sf_init(_ dbPath: UnsafePointer<CChar>?, _ dataDir: UnsafePointer<CChar>?)

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

@_silgen_name("sf_list_workflows")
func _sf_list_workflows() -> UnsafeMutablePointer<CChar>?

@_silgen_name("sf_run_bench")
func _sf_run_bench() -> UnsafeMutablePointer<CChar>?

@_silgen_name("sf_start_ideation")
func _sf_start_ideation(_ idea: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("sf_jarvis_discuss")
func _sf_jarvis_discuss(_ message: UnsafePointer<CChar>?, _ projectContext: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("sf_free_string")
func _sf_free_string(_ s: UnsafeMutablePointer<CChar>?)

// MARK: - Swift-friendly wrapper

@MainActor
final class SFBridge: ObservableObject {
    static let shared = SFBridge()

    @Published var events: [AgentEvent] = []
    @Published var isRunning = false
    @Published var currentMissionId: String?
    @Published var currentProjectId: String?
    @Published var engineReady = false
    @Published var ideationEvents: [AgentEvent] = []
    @Published var ideationRunning = false

    // Per-project event history and mission mapping
    @Published var projectEvents: [String: [AgentEvent]] = [:]
    var projectMissionIds: [String: String] = [:]

    struct AgentEvent: Identifiable {
        let id = UUID()
        let agentId: String
        let eventType: String
        let data: String
        let timestamp = Date()

        // Rich metadata (parsed from JSON for discuss_response events)
        var agentName: String = ""
        var role: String = ""
        var messageType: String = "response"
        var toAgents: [String] = []
        var round: Int = 0

        /// Parse JSON data from Rust engine's rich discussion events
        static func fromDiscussJSON(agentId: String, eventType: String, json: String) -> AgentEvent {
            var event = AgentEvent(agentId: agentId, eventType: eventType, data: "")
            guard let jsonData = json.data(using: .utf8),
                  let dict = try? JSONSerialization.jsonObject(with: jsonData) as? [String: Any] else {
                // Not JSON — use raw string as content
                event = AgentEvent(agentId: agentId, eventType: eventType, data: json)
                return event
            }
            let content = dict["content"] as? String ?? json
            let name = dict["agent_name"] as? String ?? agentId
            let role = dict["role"] as? String ?? ""
            let msgType = dict["message_type"] as? String ?? "response"
            let to = dict["to_agents"] as? [String] ?? []
            let round = dict["round"] as? Int ?? 0

            var e = AgentEvent(agentId: agentId, eventType: eventType, data: content)
            e.agentName = name
            e.role = role
            e.messageType = msgType
            e.toAgents = to
            e.round = round
            return e
        }
    }

    private init() {}

    func initialize() {
        let appSupport = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
        let sfDir = appSupport.appendingPathComponent("SimpleSF")
        let dataDir = sfDir.appendingPathComponent("data")
        try? FileManager.default.createDirectory(at: dataDir, withIntermediateDirectories: true)
        let dbPath = sfDir.appendingPathComponent("sf_engine.db").path

        // Find bundled SFData directory with agents/skills/patterns/workflows JSON
        let sfDataPath: String
        if let bundleURL = Bundle.main.url(forResource: "SFData", withExtension: nil, subdirectory: "Resources") {
            sfDataPath = bundleURL.path
        } else if let bundleURL = Bundle.main.url(forResource: "SFData", withExtension: nil) {
            sfDataPath = bundleURL.path
        } else if let moduleBundle = Bundle.main.url(forResource: "SimpleSF_SimpleSF", withExtension: "bundle"),
                  let nested = Bundle(url: moduleBundle)?.url(forResource: "SFData", withExtension: nil) {
            sfDataPath = nested.path
        } else {
            // Direct path fallback for SPM builds
            let execURL = Bundle.main.executableURL?.deletingLastPathComponent()
            let candidates = [
                execURL?.appendingPathComponent("SimpleSF_SimpleSF.bundle/SFData"),
                execURL?.deletingLastPathComponent().appendingPathComponent("Resources/SFData"),
                Bundle.main.resourceURL?.appendingPathComponent("SFData"),
            ]
            sfDataPath = candidates.compactMap { $0 }.first(where: {
                FileManager.default.fileExists(atPath: $0.appendingPathComponent("agents.json").path)
            })?.path ?? ""
            if sfDataPath.isEmpty {
                print("[SFBridge] WARNING: SFData not found in bundle — using fallback agents")
                print("[SFBridge] Searched: \(candidates.compactMap { $0?.path })")
            }
        }

        dbPath.withCString { dbPtr in
            sfDataPath.withCString { dataPtr in
                _sf_init(dbPtr, dataPtr)
            }
        }

        // Set callback (routes through a global C function)
        _sf_set_callback(sfEventHandler)

        engineReady = true
    }

    /// Synchronous config sync — used after user-initiated provider changes.
    /// NOTE: calls SecItemCopyMatching which may block if keychain is locked.
    func syncLLMConfig() {
        let state = AppState.shared
        if let provider = state.selectedProvider {
            let model = state.selectedModel.isEmpty ? provider.defaultModel : state.selectedModel
            switch provider {
            case .mlx:
                if MLXService.shared.isRunning {
                    configureLLM(provider: "mlx", apiKey: "no-key", baseUrl: MLXService.shared.baseURL,
                                 model: MLXService.shared.activeModel?.name ?? model)
                    return
                }
            case .ollama:
                if OllamaService.shared.isRunning, let m = OllamaService.shared.activeModel {
                    configureLLM(provider: "ollama", apiKey: "no-key", baseUrl: OllamaService.shared.openaiBaseURL,
                                 model: m.name)
                    return
                }
            default:
                if let apiKey = KeychainService.shared.key(for: provider) {
                    configureLLM(provider: provider.rawValue, apiKey: apiKey,
                                 baseUrl: provider.baseURL, model: model)
                    return
                }
            }
        }
        if MLXService.shared.isRunning {
            configureLLM(provider: "mlx", apiKey: "no-key", baseUrl: MLXService.shared.baseURL,
                         model: MLXService.shared.activeModel?.name ?? "mlx-local")
            return
        }
        if OllamaService.shared.isRunning, let m = OllamaService.shared.activeModel {
            configureLLM(provider: "ollama", apiKey: "no-key", baseUrl: OllamaService.shared.openaiBaseURL,
                         model: m.name)
            return
        }
        let keychain = KeychainService.shared
        guard let provider = LLMProvider.cloudProviders.first(where: { keychain.storedProviders.contains($0) }),
              let apiKey = keychain.key(for: provider) else { return }
        configureLLM(provider: provider.rawValue, apiKey: apiKey,
                     baseUrl: provider.baseURL, model: provider.defaultModel)
    }

    /// Async variant — runs keychain access on background thread.
    func syncLLMConfigAsync() async {
        let state = AppState.shared
        let keychain = KeychainService.shared

        // 1. Explicit user selection takes priority
        if let provider = state.selectedProvider {
            let model = state.selectedModel.isEmpty ? provider.defaultModel : state.selectedModel
            switch provider {
            case .mlx:
                let svc = MLXService.shared
                if svc.isRunning {
                    configureLLM(provider: "mlx", apiKey: "no-key", baseUrl: svc.baseURL,
                                 model: svc.activeModel?.name ?? model)
                    return
                }
            case .ollama:
                let svc = OllamaService.shared
                if svc.isRunning, let m = svc.activeModel {
                    configureLLM(provider: "ollama", apiKey: "no-key", baseUrl: svc.openaiBaseURL,
                                 model: m.name)
                    return
                }
            default:
                if keychain.storedProviders.contains(provider) {
                    let svc = keychain.service
                    let raw = provider.rawValue
                    let apiKey: String? = await Task.detached(operation: {
                        Self.keychainLookup(service: svc, account: raw)
                    }).value
                    if let apiKey {
                        configureLLM(provider: provider.rawValue, apiKey: apiKey,
                                     baseUrl: provider.baseURL, model: model)
                        return
                    }
                }
            }
        }

        // 2. Local providers
        let preferred = state.preferredLocalProvider
        if preferred == "mlx", MLXService.shared.isRunning {
            configureLLM(provider: "mlx", apiKey: "no-key", baseUrl: MLXService.shared.baseURL,
                         model: MLXService.shared.activeModel?.name ?? "mlx-local")
            return
        }
        if preferred == "ollama", OllamaService.shared.isRunning, let m = OllamaService.shared.activeModel {
            configureLLM(provider: "ollama", apiKey: "no-key", baseUrl: OllamaService.shared.openaiBaseURL,
                         model: m.name)
            return
        }
        if MLXService.shared.isRunning {
            configureLLM(provider: "mlx", apiKey: "no-key", baseUrl: MLXService.shared.baseURL,
                         model: MLXService.shared.activeModel?.name ?? "mlx-local")
            return
        }
        if OllamaService.shared.isRunning, let m = OllamaService.shared.activeModel {
            configureLLM(provider: "ollama", apiKey: "no-key", baseUrl: OllamaService.shared.openaiBaseURL,
                         model: m.name)
            return
        }

        // 3. First cloud with key
        guard let provider = LLMProvider.cloudProviders.first(where: { keychain.storedProviders.contains($0) }) else { return }
        let svc = keychain.service
        let raw = provider.rawValue
        let apiKey: String? = await Task.detached(operation: {
            Self.keychainLookup(service: svc, account: raw)
        }).value
        guard let apiKey else { return }
        configureLLM(provider: provider.rawValue, apiKey: apiKey,
                     baseUrl: provider.baseURL, model: provider.defaultModel)
    }

    /// Thread-safe keychain lookup (no @MainActor)
    private nonisolated static func keychainLookup(service: String, account: String) -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        var item: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &item) == errSecSuccess,
              let data = item as? Data,
              let str = String(data: data, encoding: .utf8), !str.isEmpty
        else { return nil }
        return str
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

    /// Non-blocking mission start — runs FFI call on background thread.
    func startMissionAsync(projectId: String, brief: String) {
        // Store events for project before clearing
        events.removeAll()
        isRunning = true
        currentProjectId = projectId
        projectEvents[projectId] = []   // fresh events for this project
        let pid = projectId
        let b = brief
        Task.detached {
            var missionId: String?
            pid.withCString { p in
                b.withCString { bb in
                    if let ptr = _sf_start_mission(p, bb) {
                        missionId = String(cString: ptr)
                        _sf_free_string(ptr)
                    }
                }
            }
            await MainActor.run {
                SFBridge.shared.currentMissionId = missionId
                if let mid = missionId {
                    SFBridge.shared.projectMissionIds[pid] = mid
                }
            }
        }
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
        return missionStatusById(mid)
    }

    /// Get mission status for a specific project (uses stored mission ID mapping)
    func missionStatusForProject(_ projectId: String) -> MissionStatus? {
        guard let mid = projectMissionIds[projectId] else { return nil }
        return missionStatusById(mid)
    }

    /// Get events for a specific project
    func eventsForProject(_ projectId: String) -> [AgentEvent] {
        return projectEvents[projectId] ?? []
    }

    private func missionStatusById(_ mid: String) -> MissionStatus? {
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

    func listWorkflows() -> [[String: Any]] {
        guard let ptr = _sf_list_workflows() else { return [] }
        let json = String(cString: ptr)
        _sf_free_string(ptr)
        guard let data = json.data(using: .utf8),
              let arr = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] else { return [] }
        return arr
    }

    /// Run AC/LLM bench tests. Returns JSON array of results.
    func runBench() -> String {
        syncLLMConfig()
        guard let ptr = _sf_run_bench() else { return "[]" }
        let result = String(cString: ptr)
        _sf_free_string(ptr)
        return result
    }

    func startIdeation(idea: String) -> String? {
        ideationEvents.removeAll()
        ideationRunning = true
        syncLLMConfig()
        var result: String?
        idea.withCString { ptr in
            if let r = _sf_start_ideation(ptr) {
                result = String(cString: r)
                _sf_free_string(r)
            }
        }
        return result
    }

    /// Start a Jarvis intake discussion (network pattern: RTE + PO discuss the request).
    /// Events stream via the callback with "discuss_*" event types.
    @Published var discussionEvents: [AgentEvent] = []
    @Published var discussionRunning = false
    @Published var discussionSynthesis: String?

    func startDiscussion(message: String, projectContext: String) -> String? {
        discussionEvents.removeAll()
        discussionRunning = true
        discussionSynthesis = nil
        syncLLMConfig()
        var result: String?
        message.withCString { m in
            projectContext.withCString { c in
                if let r = _sf_jarvis_discuss(m, c) {
                    result = String(cString: r)
                    _sf_free_string(r)
                }
            }
        }
        return result
    }

    /// Non-blocking variant: runs the FFI call on a background thread.
    func startDiscussionAsync(message: String, projectContext: String) {
        discussionEvents.removeAll()
        discussionRunning = true
        discussionSynthesis = nil
        let msg = message
        let ctx = projectContext
        Task.detached {
            msg.withCString { m in
                ctx.withCString { c in
                    let _ = _sf_jarvis_discuss(m, c)
                }
            }
        }
    }

    // Called from the global C callback
    nonisolated func handleEvent(agentId: String, eventType: String, data: String) {
        Task { @MainActor in
            switch eventType {
            // Discussion events (Jarvis intake) — data is JSON for discuss_response
            case "discuss_thinking":
                let event = AgentEvent(agentId: agentId, eventType: eventType, data: data)
                self.discussionEvents.append(event)
            case "discuss_response":
                let event = AgentEvent.fromDiscussJSON(agentId: agentId, eventType: eventType, json: data)
                self.discussionEvents.append(event)
            case "discuss_complete":
                self.discussionSynthesis = data
                self.discussionRunning = false

            // Ideation events
            case "ideation_response":
                let event = AgentEvent(agentId: agentId, eventType: eventType, data: data)
                self.ideationEvents.append(event)
            case "ideation_complete":
                self.ideationRunning = false

            // Mission events
            case "mission_complete":
                let event = AgentEvent(agentId: agentId, eventType: eventType, data: data)
                self.events.append(event)
                if let pid = self.currentProjectId {
                    self.projectEvents[pid, default: []].append(event)
                }
                self.isRunning = false
            case "error":
                let event = AgentEvent(agentId: agentId, eventType: eventType, data: data)
                self.events.append(event)
                if let pid = self.currentProjectId {
                    self.projectEvents[pid, default: []].append(event)
                }
                if self.discussionRunning { self.discussionRunning = false }
            default:
                let event = AgentEvent(agentId: agentId, eventType: eventType, data: data)
                if agentId == "engine" && data.hasPrefix("---") {
                    self.ideationEvents.append(event)
                } else {
                    self.events.append(event)
                    if let pid = self.currentProjectId {
                        self.projectEvents[pid, default: []].append(event)
                    }
                }
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
