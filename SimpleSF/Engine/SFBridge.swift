import Foundation

// MARK: - C FFI declarations (core engine lifecycle)

// We declare the C functions directly since SPM doesn't easily support bridging headers.
// These match the extern "C" functions in SFEngine/src/ffi.rs.

// Ref: FT-SSF-002
typealias SFEventCallback = @convention(c) (UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafePointer<CChar>?) -> Void

@_silgen_name("sf_init")
func _sf_init(_ dbPath: UnsafePointer<CChar>?, _ dataDir: UnsafePointer<CChar>?)

@_silgen_name("sf_set_callback")
func _sf_set_callback(_ cb: SFEventCallback)

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

    // Discussion state
    @Published var discussionEvents: [AgentEvent] = []
    @Published var discussionRunning = false
    @Published var discussionSynthesis: String?
    @Published var isReasoning = false

    /// Path to the Rust engine SQLite DB (set during initialize)
    var dbPath: String = ""

    /// Agent metadata cache (populated after first listAgents call)
    var _agentCache: [String: SFAgent] = [:]

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

    // MARK: - Persistence

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
        // Migrate orphaned events to current projects (UUID changed but name matches)
        _migrateOrphanedEvents()
    }

    /// If event files exist for project IDs not in ProjectStore, migrate them
    /// to the best matching current project (preferring active projects with fewer events).
    private func _migrateOrphanedEvents() {
        let knownIds = Set(ProjectStore.shared.projects.map(\.id))
        let orphanedIds = projectEvents.keys.filter { !knownIds.contains($0) }
        guard !orphanedIds.isEmpty else { return }

        for orphanId in orphanedIds {
            guard let orphanEvents = projectEvents[orphanId], !orphanEvents.isEmpty else { continue }

            // Find best target: active project with fewest events (most likely the recreated one)
            let activeProjects = ProjectStore.shared.projects.filter { $0.status == .active }
            let target = activeProjects.min(by: { a, b in
                (projectEvents[a.id]?.count ?? 0) < (projectEvents[b.id]?.count ?? 0)
            })

            if let target = target {
                let existing = projectEvents[target.id] ?? []
                if orphanEvents.count > existing.count {
                    // Orphan has richer history — use it (prepend existing after orphan)
                    projectEvents[target.id] = orphanEvents + existing
                    print("[SFBridge] Migrated \(orphanEvents.count) orphaned events → \(target.name) (\(target.id.prefix(8)))")
                }
            }
            // Remove orphan entry and delete orphan file
            projectEvents.removeValue(forKey: orphanId)
            let orphanFile = eventsDir.appendingPathComponent("\(orphanId).json")
            try? FileManager.default.removeItem(at: orphanFile)
        }
        // Re-persist with corrected IDs
        _persistProjectEvents()
    }

    // MARK: - Discussion History (from DB)

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

    // MARK: - AgentEvent

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

    // MARK: - Initialize

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

    // MARK: - Event Routing

    /// Get events for a specific project
    func eventsForProject(_ projectId: String) -> [AgentEvent] {
        let perProject = projectEvents[projectId] ?? []
        if !perProject.isEmpty { return perProject }
        // Fallback: if this is the current project, use global events
        if projectId == currentProjectId { return events }
        return []
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
                    // Persist after each response so conversations survive crashes
                    if eventType == "response" {
                        self._persistProjectEvents()
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

}

// Global C callback function — routes to SFBridge singleton
private func sfEventHandler(agentId: UnsafePointer<CChar>?, eventType: UnsafePointer<CChar>?, data: UnsafePointer<CChar>?) {
    let a = agentId.map { String(cString: $0) } ?? ""
    let t = eventType.map { String(cString: $0) } ?? ""
    let d = data.map { String(cString: $0) } ?? ""
    SFBridge.shared.handleEvent(agentId: a, eventType: t, data: d)
}
