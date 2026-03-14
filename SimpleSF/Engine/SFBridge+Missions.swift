import Foundation

// Ref: FT-SSF-002

// MARK: - C FFI declarations (mission lifecycle)

@_silgen_name("sf_start_mission")
func _sf_start_mission(_ projectId: UnsafePointer<CChar>?, _ brief: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("sf_mission_status")
func _sf_mission_status(_ missionId: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

// MARK: - Mission Operations

extension SFBridge {

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
        let phase_type: String?
        let iteration: Int?
        let max_iterations: Int?
    }

    struct MessageInfo: Codable, Identifiable {
        var id: String { "\(agent_name)-\(created_at)" }
        let agent_name: String
        let role: String
        let content: String
        let tool_calls: String?
        let created_at: String
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
        events.removeAll()
        isRunning = true
        currentProjectId = projectId
        projectEvents[projectId] = []   // fresh events for this project
        _launchMission(projectId: projectId, brief: brief)
    }

    /// Resume a mission without clearing existing conversation events.
    func resumeMissionAsync(projectId: String, brief: String) {
        events.removeAll()
        isRunning = true
        currentProjectId = projectId
        // Add a separator event so user knows where the resume starts
        let sep = AgentEvent(agentId: "engine", eventType: "response", data: "── Reprise de la mission ──")
        projectEvents[projectId, default: []].append(sep)
        _launchMission(projectId: projectId, brief: brief)
    }

    /// Shared mission launch logic.
    private func _launchMission(projectId: String, brief: String) {
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

    private func missionStatusById(_ mid: String) -> MissionStatus? {
        return mid.withCString { ptr -> MissionStatus? in
            guard let result = _sf_mission_status(ptr) else { return nil }
            let json = String(cString: result)
            _sf_free_string(result)
            guard let data = json.data(using: .utf8) else { return nil }
            return try? JSONDecoder().decode(MissionStatus.self, from: data)
        }
    }

    /// Discover mission ID for active projects and restore their conversation from DB.
    func bootstrapActiveProject() {
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

            // Restore conversation from DB if we don't have events in memory/disk cache
            _restoreConversationFromDB(projectId: project.id)
        }

        // Auto-resume: restart the first active project (preserving existing conversation)
        if !isRunning, let project = activeProjects.first {
            print("[SFBridge] Auto-resuming project: \(project.name)")
            currentProjectId = project.id
            Task {
                await KeychainService.shared.scanIfNeeded()
                await syncLLMConfigAsync()
                resumeMissionAsync(projectId: project.id, brief: project.description)
            }
        }
    }

    /// Load conversation messages from the Rust DB into projectEvents (if not already populated)
    private func _restoreConversationFromDB(projectId: String) {
        // Skip if we already have events from disk cache or live session
        if let existing = projectEvents[projectId], !existing.isEmpty { return }

        guard let status = missionStatusForProject(projectId),
              !status.messages.isEmpty else { return }

        var restored: [AgentEvent] = []
        // Messages come in reverse order (DESC) from DB — reverse to chronological
        for msg in status.messages.reversed() {
            guard msg.role == "assistant", !msg.content.isEmpty else { continue }
            var event = AgentEvent(agentId: msg.agent_name, eventType: "response", data: msg.content)
            event.agentName = msg.agent_name
            event.role = msg.role
            restored.append(event)
        }

        if !restored.isEmpty {
            projectEvents[projectId] = restored
            print("[SFBridge] Restored \(restored.count) messages from DB for project \(projectId)")
        }
    }
}
