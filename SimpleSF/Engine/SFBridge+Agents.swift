import Foundation

// Ref: FT-SSF-002

// MARK: - C FFI declarations (agents, workflows, bench, ideation)

@_silgen_name("sf_list_agents")
func _sf_list_agents() -> UnsafeMutablePointer<CChar>?

@_silgen_name("sf_list_workflows")
func _sf_list_workflows() -> UnsafeMutablePointer<CChar>?

@_silgen_name("sf_run_bench")
func _sf_run_bench() -> UnsafeMutablePointer<CChar>?

@_silgen_name("sf_start_ideation")
func _sf_start_ideation(_ idea: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

// MARK: - Agent, Workflow & Ideation Operations

extension SFBridge {

    struct SFAgent: Codable, Identifiable {
        let id: String
        let name: String
        let role: String
        let persona: String
    }

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
}
