import Foundation
import Security
import SQLite3

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

@_silgen_name("sf_set_yolo")
func _sf_set_yolo(_ enabled: Bool)

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

@_silgen_name("sf_load_discussion_history")
func _sf_load_discussion_history() -> UnsafeMutablePointer<CChar>?

@_silgen_name("sf_free_string")
func _sf_free_string(_ s: UnsafeMutablePointer<CChar>?)

// MARK: - Swift-friendly wrapper

@MainActor
final class SFBridge: ObservableObject {
    static let shared = SFBridge()

    @Published var events: [AgentEvent] = []
    @Published var isRunning = false
    @Published var currentMissionId: String?
    @Published var currentProjectId: String? {
        didSet { UserDefaults.standard.set(currentProjectId, forKey: "sf_current_project_id") }
    }
    @Published var engineReady = false
    @Published var ideationEvents: [AgentEvent] = []
    @Published var ideationRunning = false

    // Per-project event history and mission mapping
    @Published var projectEvents: [String: [AgentEvent]] = [:]
    var projectMissionIds: [String: String] {
        didSet { _persistMissionIds() }
    }

    /// Path to the Rust engine SQLite DB (set during initialize)
    private var dbPath: String = ""

    private static let missionIdsKey = "sf_project_mission_ids"

    private var eventsDir: URL {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
            .appendingPathComponent("SimpleSF", isDirectory: true)
            .appendingPathComponent("events", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir
    }

    private init() {
        // Restore persisted state
        projectMissionIds = UserDefaults.standard.dictionary(forKey: Self.missionIdsKey) as? [String: String] ?? [:]
        currentProjectId = UserDefaults.standard.string(forKey: "sf_current_project_id")
        _loadProjectEvents()
        // Note: _loadDiscussionHistory() is called later in initialize() after DB is ready
    }

    private func _persistMissionIds() {
        UserDefaults.standard.set(projectMissionIds, forKey: Self.missionIdsKey)
    }

    private func _persistProjectEvents() {
        let dir = eventsDir
        let enc = JSONEncoder()
        for (projectId, events) in projectEvents where !events.isEmpty {
            let url = dir.appendingPathComponent("\(projectId).json")
            if let data = try? enc.encode(events) {
                try? data.write(to: url, options: .atomic)
            }
        }
    }

    private func _loadProjectEvents() {
        let dir = eventsDir
        let dec = JSONDecoder()
        guard let files = try? FileManager.default.contentsOfDirectory(at: dir, includingPropertiesForKeys: nil)
        else { return }
        for file in files where file.pathExtension == "json" {
            let projectId = file.deletingPathExtension().lastPathComponent
            if let data = try? Data(contentsOf: file),
               let events = try? dec.decode([AgentEvent].self, from: data), !events.isEmpty {
                projectEvents[projectId] = events
            }
        }
    }

    /// Restore the most recent discussion from the Rust DB into discussionEvents
    private func _loadDiscussionHistory() {
        guard let rawPtr = _sf_load_discussion_history() else { return }
        let json = String(cString: rawPtr)
        _sf_free_string(rawPtr)

        guard let data = json.data(using: .utf8),
              let msgs = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]],
              !msgs.isEmpty else { return }

        var restored: [AgentEvent] = []
        for msg in msgs {
            let agentId = msg["agent_id"] as? String ?? "unknown"
            let content = msg["content"] as? String ?? ""
            let agentName = msg["agent_name"] as? String ?? ""
            let role = msg["role"] as? String ?? ""
            let round = msg["round"] as? Int ?? 0

            // Reconstruct as discuss_response JSON (same format as live events)
            let richJSON = """
            {"content":\(Self._jsonEscape(content)),"agent_name":"\(Self._jsonEscape(agentName))","role":"\(Self._jsonEscape(role))","message_type":"response","to_agents":[],"round":\(round)}
            """
            let event = AgentEvent.fromDiscussJSON(agentId: agentId, eventType: "discuss_response", json: richJSON)
            restored.append(event)
        }

        discussionEvents = restored
    }

    private static func _jsonEscape(_ s: String) -> String {
        // For embedding in JSON string values
        let escaped = s.replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "\"", with: "\\\"")
            .replacingOccurrences(of: "\n", with: "\\n")
            .replacingOccurrences(of: "\r", with: "\\r")
            .replacingOccurrences(of: "\t", with: "\\t")
        return "\"\(escaped)\""
    }

    struct AgentEvent: Identifiable, Codable {
        let id: UUID
        let agentId: String
        let eventType: String
        var data: String
        let timestamp: Date

        // Rich metadata (parsed from JSON for discuss_response events)
        var agentName: String = ""
        var role: String = ""
        var messageType: String = "response"
        var toAgents: [String] = []
        var round: Int = 0

        init(agentId: String, eventType: String, data: String) {
            self.id = UUID()
            self.agentId = agentId
            self.eventType = eventType
            self.data = data
            self.timestamp = Date()
        }

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

    func initialize() {
        let appSupport = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
        let sfDir = appSupport.appendingPathComponent("SimpleSF")
        let dataDir = sfDir.appendingPathComponent("data")
        try? FileManager.default.createDirectory(at: dataDir, withIntermediateDirectories: true)
        let dbPath = sfDir.appendingPathComponent("sf_engine.db").path
        self.dbPath = dbPath

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

        // Sync settings to engine
        syncYoloMode()

        engineReady = true

        // Restore discussion history now that DB is ready
        _loadDiscussionHistory()

        // Bootstrap: if a project is active but has no mission mapping, discover it
        bootstrapActiveProject()
    }

    /// Discover mission ID for active projects and auto-resume them.
    private func bootstrapActiveProject() {
        let activeProjects = ProjectStore.shared.projects.filter { $0.status == .active }
        guard !activeProjects.isEmpty else { return }

        for project in activeProjects {
            if currentProjectId == nil {
                currentProjectId = project.id
            }
            // Try to recover existing mission ID
            if project.missionId == nil && projectMissionIds[project.id] == nil {
                let rustProjects = listProjects()
                if let rustProject = rustProjects.first(where: { $0.name == project.name }),
                   let _ = missionStatusById(rustProject.id) {
                    projectMissionIds[project.id] = rustProject.id
                    ProjectStore.shared.setMissionId(project.id, missionId: rustProject.id)
                    print("[SFBridge] Bootstrapped mission \(rustProject.id) for project \(project.name)")
                }
            }
        }

        // Auto-resume: restart the first active project if nothing is running
        if !isRunning, let project = activeProjects.first {
            print("[SFBridge] Auto-resuming project: \(project.name)")
            currentProjectId = project.id
            projectEvents[project.id] = []
            Task {
                // Ensure keychain is scanned before LLM config
                await KeychainService.shared.scanIfNeeded()
                await syncLLMConfigAsync()
                startMissionAsync(projectId: project.id, brief: project.description)
            }
        }
    }

    /// Sync YOLO mode to the Rust engine
    func syncYoloMode() {
        _sf_set_yolo(AppState.shared.yoloMode)
    }

    /// Synchronous config sync — used after user-initiated provider changes.
    /// NOTE: calls SecItemCopyMatching which may block if keychain is locked.
    func syncLLMConfig() {
        let state = AppState.shared
        if let provider = state.selectedProvider {
            let model = state.selectedModel.isEmpty ? provider.defaultModel : state.selectedModel
            switch provider {
            case .mlx:
                // Trust user selection — configure MLX even if not confirmed running yet.
                // If server is down, Rust fails fast with connection-refused.
                configureLLM(provider: "mlx", apiKey: "no-key", baseUrl: MLXService.shared.baseURL,
                             model: MLXService.shared.activeModel?.name ?? model)
                return
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
                configureLLM(provider: "mlx", apiKey: "no-key", baseUrl: svc.baseURL,
                             model: svc.activeModel?.name ?? model)
                return
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
        currentProjectId = projectId
        projectEvents[projectId] = []
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
        if let mid = result {
            projectMissionIds[projectId] = mid
            ProjectStore.shared.setMissionId(projectId, missionId: mid)
        }
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
                    ProjectStore.shared.setMissionId(pid, missionId: mid)
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
        // 1. In-memory mapping (set during startMissionAsync in this session)
        if let mid = projectMissionIds[projectId] {
            return missionStatusById(mid)
        }
        // 2. Persisted in Project model (survives app restarts)
        if let mid = ProjectStore.shared.projects.first(where: { $0.id == projectId })?.missionId {
            projectMissionIds[projectId] = mid   // cache for next call
            return missionStatusById(mid)
        }
        // 3. Fallback: if this is the current project, use global mission ID
        if projectId == currentProjectId, let mid = currentMissionId {
            return missionStatusById(mid)
        }
        return nil
    }

    /// Get events for a specific project
    func eventsForProject(_ projectId: String) -> [AgentEvent] {
        let perProject = projectEvents[projectId] ?? []
        if !perProject.isEmpty { return perProject }
        // Fallback: if this is the current project, use global events
        if projectId == currentProjectId { return events }
        return []
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

    private var _agentCache: [String: SFAgent] = [:]

    /// Quick lookup for agent metadata (cached after first listAgents call)
    func agentInfo(_ agentId: String) -> SFAgent? {
        if _agentCache.isEmpty {
            for a in listAgents() { _agentCache[a.id] = a }
        }
        return _agentCache[agentId]
    }

    func listAgents() -> [SFAgent] {
        guard let ptr = _sf_list_agents() else { return [] }
        let json = String(cString: ptr)
        _sf_free_string(ptr)
        guard let data = json.data(using: .utf8) else { return [] }
        let agents = (try? JSONDecoder().decode([SFAgent].self, from: data)) ?? []
        for a in agents { _agentCache[a.id] = a }
        return agents
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
    @Published var isReasoning = false

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
            // Discussion events (Jarvis intake) — Jarvis-only, never leak to project
            case "discuss_thinking":
                let event = AgentEvent(agentId: agentId, eventType: eventType, data: data)
                self.discussionEvents.append(event)
            case "discuss_reasoning":
                self.isReasoning = (data == "start")
            case "discuss_response":
                self.discussionEvents.removeAll { $0.agentId == agentId && $0.eventType == "discuss_thinking" }
                self.isReasoning = false
                let event = AgentEvent.fromDiscussJSON(agentId: agentId, eventType: eventType, json: data)
                self.discussionEvents.append(event)
            case "discuss_chunk":
                self._appendChunk(agentId: agentId, chunk: data, to: &self.discussionEvents, eventType: "discuss_response")
            case "discuss_complete":
                self.discussionSynthesis = data
                self.discussionRunning = false
                self._persistProjectEvents()

            // Ideation events
            case "ideation_reasoning":
                self.isReasoning = (data == "start")
            case "ideation_response":
                let event = AgentEvent(agentId: agentId, eventType: eventType, data: data)
                self.ideationEvents.append(event)
            case "ideation_chunk":
                self._appendChunk(agentId: agentId, chunk: data, to: &self.ideationEvents, eventType: "ideation_response")
            case "ideation_complete":
                self.ideationRunning = false

            // Streaming chunks for mission/project events
            case "reasoning":
                self.isReasoning = (data == "start")
            case "response_chunk":
                self._appendChunk(agentId: agentId, chunk: data, to: &self.events, eventType: "response")
                if let pid = self.currentProjectId {
                    self._appendChunk(agentId: agentId, chunk: data, to: &self.projectEvents[pid, default: []], eventType: "response")
                }

            // Mission events
            case "mission_complete":
                let event = AgentEvent(agentId: agentId, eventType: eventType, data: data)
                self.events.append(event)
                if let pid = self.currentProjectId {
                    self.projectEvents[pid, default: []].append(event)
                }
                self.isRunning = false
                self._persistProjectEvents()
            case "error":
                let event = AgentEvent(agentId: agentId, eventType: eventType, data: data)
                self.events.append(event)
                if let pid = self.currentProjectId {
                    self.projectEvents[pid, default: []].append(event)
                }
                if self.discussionRunning { self.discussionRunning = false }
                self._persistProjectEvents()
            default:
                // Try to parse as rich JSON (agent name, role, recipients)
                let event: AgentEvent
                if eventType == "response" || eventType == "tool_call" || eventType == "tool_result" || eventType == "thinking" {
                    let parsed = AgentEvent.fromDiscussJSON(agentId: agentId, eventType: eventType, json: data)
                    if !parsed.agentName.isEmpty {
                        event = parsed
                    } else {
                        // Plain text — enrich from catalog
                        var e = AgentEvent(agentId: agentId, eventType: eventType, data: data)
                        if let info = self.agentInfo(agentId) {
                            e.agentName = info.name
                            e.role = info.role
                        }
                        event = e
                    }
                } else {
                    event = AgentEvent(agentId: agentId, eventType: eventType, data: data)
                }

                // Clear thinking spinners when a substantive event arrives for this agent
                if eventType == "response" || eventType == "tool_call" {
                    self.events.removeAll { $0.agentId == agentId && $0.eventType == "thinking" }
                    if let pid = self.currentProjectId {
                        self.projectEvents[pid, default: []].removeAll { $0.agentId == agentId && $0.eventType == "thinking" }
                    }
                    self.isReasoning = false
                }

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

    /// Append a streaming chunk to the last event from this agent, or create a new one.
    /// On first chunk, removes any thinking/reasoning indicators for this agent.
    @MainActor
    private func _appendChunk(agentId: String, chunk: String, to events: inout [AgentEvent], eventType: String) {
        if let idx = events.lastIndex(where: { $0.agentId == agentId && $0.eventType == eventType }) {
            events[idx].data += chunk
        } else {
            // First chunk for this agent — remove stale thinking spinners
            events.removeAll { $0.agentId == agentId && ($0.eventType == "thinking" || $0.eventType == "discuss_thinking") }
            self.isReasoning = false

            var event = AgentEvent(agentId: agentId, eventType: eventType, data: chunk)
            if let info = self.agentInfo(agentId) {
                event.agentName = info.name
                event.role = info.role
            }
            events.append(event)
        }
    }

    // MARK: - Discussion Messages from DB

    struct DiscussionMessage: Identifiable {
        let id: Int
        let sessionId: String
        let agentId: String
        let agentName: String
        let agentRole: String
        let round: Int
        let content: String
        let createdAt: String
    }

    /// Read all discussion sessions from the Rust engine DB
    func listDiscussionSessions() -> [(id: String, topic: String, status: String)] {
        guard !dbPath.isEmpty else { return [] }
        var db: OpaquePointer?
        guard sqlite3_open_v2(dbPath, &db, SQLITE_OPEN_READONLY, nil) == SQLITE_OK else { return [] }
        defer { sqlite3_close(db) }

        var stmt: OpaquePointer?
        let sql = "SELECT id, topic, status FROM discussion_sessions ORDER BY created_at DESC"
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return [] }
        defer { sqlite3_finalize(stmt) }

        var results: [(id: String, topic: String, status: String)] = []
        while sqlite3_step(stmt) == SQLITE_ROW {
            let id = String(cString: sqlite3_column_text(stmt, 0))
            let topic = String(cString: sqlite3_column_text(stmt, 1))
            let status = String(cString: sqlite3_column_text(stmt, 2))
            results.append((id, topic, status))
        }
        return results
    }

    /// Read discussion messages for a given session ID
    func discussionMessages(sessionId: String) -> [DiscussionMessage] {
        guard !dbPath.isEmpty else { return [] }
        var db: OpaquePointer?
        guard sqlite3_open_v2(dbPath, &db, SQLITE_OPEN_READONLY, nil) == SQLITE_OK else { return [] }
        defer { sqlite3_close(db) }

        var stmt: OpaquePointer?
        let sql = """
            SELECT id, session_id, agent_id, agent_name, agent_role, round, content, created_at
            FROM discussion_messages WHERE session_id = ? ORDER BY id
            """
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return [] }
        defer { sqlite3_finalize(stmt) }
        sqlite3_bind_text(stmt, 1, sessionId, -1, unsafeBitCast(-1, to: sqlite3_destructor_type.self))

        var msgs: [DiscussionMessage] = []
        while sqlite3_step(stmt) == SQLITE_ROW {
            msgs.append(DiscussionMessage(
                id: Int(sqlite3_column_int(stmt, 0)),
                sessionId: String(cString: sqlite3_column_text(stmt, 1)),
                agentId: String(cString: sqlite3_column_text(stmt, 2)),
                agentName: String(cString: sqlite3_column_text(stmt, 3)),
                agentRole: String(cString: sqlite3_column_text(stmt, 4)),
                round: Int(sqlite3_column_int(stmt, 5)),
                content: String(cString: sqlite3_column_text(stmt, 6)),
                createdAt: String(cString: sqlite3_column_text(stmt, 7))
            ))
        }
        return msgs
    }

    /// Find the most recent discussion session matching a project name/topic
    func discussionSessionForProject(_ projectName: String) -> String? {
        let sessions = listDiscussionSessions()
        let lowered = projectName.lowercased()
        return sessions.first(where: { $0.topic.lowercased().contains(lowered) })?.id
    }

    /// Find discussion messages for a project by matching its name in session topics
    func discussionMessagesForProject(_ projectName: String) -> [DiscussionMessage] {
        guard let sessionId = discussionSessionForProject(projectName) else { return [] }
        return discussionMessages(sessionId: sessionId)
    }

    /// Get messages from the most recent discussion session that has messages
    func mostRecentDiscussionMessages() -> [DiscussionMessage] {
        let sessions = listDiscussionSessions()
        for session in sessions {
            let msgs = discussionMessages(sessionId: session.id)
            if !msgs.isEmpty { return msgs }
        }
        return []
    }
}

// Global C callback function — routes to SFBridge singleton
private func sfEventHandler(agentId: UnsafePointer<CChar>?, eventType: UnsafePointer<CChar>?, data: UnsafePointer<CChar>?) {
    let a = agentId.map { String(cString: $0) } ?? ""
    let t = eventType.map { String(cString: $0) } ?? ""
    let d = data.map { String(cString: $0) } ?? ""
    SFBridge.shared.handleEvent(agentId: a, eventType: t, data: d)
}
