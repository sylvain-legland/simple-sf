import Foundation
import SQLite3

// Ref: FT-SSF-002

// MARK: - C FFI declarations (discussion / Jarvis intake)

@_silgen_name("sf_jarvis_discuss")
func _sf_jarvis_discuss(_ message: UnsafePointer<CChar>?, _ projectContext: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

// MARK: - Discussion Operations

extension SFBridge {

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

    /// Start a Jarvis intake discussion (network pattern: RTE + PO discuss the request).
    /// Events stream via the callback with "discuss_*" event types.
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

    // MARK: - Discussion Messages from DB

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
