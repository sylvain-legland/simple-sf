import SwiftUI

// MARK: - Action parsing (Jarvis manages projects)

// Ref: FT-SSF-001
enum JarvisAction {
    case createProject(name: String, description: String, tech: String)
    case deleteProject(name: String)
    case updateProject(name: String, status: ProjectStatus?)
    case startMission(projectName: String, brief: String)

    static func parse(_ text: String) -> [JarvisAction] {
        var actions: [JarvisAction] = []

        func attr(_ key: String, in block: String) -> String? {
            guard let range = block.range(of: "\(key)=\"") else { return nil }
            let start = range.upperBound
            guard let end = block[start...].firstIndex(of: "\"") else { return nil }
            return String(block[start..<end])
        }

        var i = text.startIndex
        while i < text.endIndex {
            guard let open = text[i...].firstIndex(of: "[") else { break }
            guard let close = text[open...].firstIndex(of: "]") else { break }
            let block = String(text[text.index(after: open)..<close])

            if block.hasPrefix("CREATE_PROJECT"), let name = attr("name", in: block) {
                actions.append(.createProject(
                    name: name,
                    description: attr("description", in: block) ?? "",
                    tech: attr("tech", in: block) ?? ""
                ))
            } else if block.hasPrefix("DELETE_PROJECT"), let name = attr("name", in: block) {
                actions.append(.deleteProject(name: name))
            } else if block.hasPrefix("UPDATE_PROJECT"), let name = attr("name", in: block) {
                let status = attr("status", in: block).flatMap { ProjectStatus(rawValue: $0) }
                actions.append(.updateProject(name: name, status: status))
            } else if block.hasPrefix("START_MISSION"), let project = attr("project", in: block) {
                actions.append(.startMission(
                    projectName: project,
                    brief: attr("brief", in: block) ?? ""
                ))
            }

            i = text.index(after: close)
        }
        return actions
    }

    @MainActor func execute() {
        let store = ProjectStore.shared
        let bridge = SFBridge.shared
        switch self {
        case .createProject(let name, let desc, let tech):
            if !store.projects.contains(where: { $0.name.lowercased() == name.lowercased() }) {
                store.add(Project(name: name, description: desc, tech: tech))
            }
            let _ = bridge.createProject(name: name, description: desc, tech: tech)
        case .deleteProject(let name):
            if let p = store.projects.first(where: { $0.name.lowercased() == name.lowercased() }) {
                store.delete(p.id)
            }
            let rustProjects = bridge.listProjects()
            if let rp = rustProjects.first(where: { $0.name.lowercased() == name.lowercased() }) {
                bridge.deleteProject(id: rp.id)
            }
        case .updateProject(let name, let status):
            if let p = store.projects.first(where: { $0.name.lowercased() == name.lowercased() }),
               let s = status {
                store.setStatus(p.id, status: s)
            }
        case .startMission(let projectName, let brief):
            var rustProjects = bridge.listProjects()
            var projectId = rustProjects.first(where: { $0.name.lowercased() == projectName.lowercased() })?.id
            if projectId == nil {
                if let swiftProj = store.projects.first(where: { $0.name.lowercased() == projectName.lowercased() }) {
                    projectId = bridge.createProject(name: swiftProj.name, description: swiftProj.description, tech: swiftProj.tech)
                } else {
                    projectId = bridge.createProject(name: projectName, description: brief, tech: "")
                }
            }
            if let pid = projectId {
                bridge.syncLLMConfig()
                let _ = bridge.startMission(projectId: pid, brief: brief)
            }
        }
    }

    static func cleanDisplay(_ text: String) -> String {
        var out = text
        while let open = out.range(of: "[CREATE_PROJECT ") ?? out.range(of: "[DELETE_PROJECT ") ?? out.range(of: "[UPDATE_PROJECT ") ?? out.range(of: "[START_MISSION ") {
            if let close = out[open.lowerBound...].firstIndex(of: "]") {
                out.removeSubrange(open.lowerBound...close)
            } else { break }
        }
        return out.trimmingCharacters(in: .whitespacesAndNewlines)
    }
}

// Agent info is loaded from SFCatalog (192 agents from platform JSON)

// MARK: - JarvisView

@MainActor
struct JarvisView: View {
    @ObservedObject private var llm = LLMService.shared
    @ObservedObject var bridge = SFBridge.shared
    @ObservedObject var chatStore = ChatStore.shared
    @ObservedObject private var projects = ProjectStore.shared
    @ObservedObject var catalog = SFCatalog.shared

    @State private var inputText = ""
    @State private var isProcessing = false
    @State private var errorMessage: String?
    @State private var chatLoadingState: LoadingState = .loading  // Ref: FT-SSF-013

    var session: ChatSession? { chatStore.activeSession }
    private var messages: [LLMMessage] { session?.messages ?? [] }

    var body: some View {
        HSplitView {
            sessionSidebar
                .frame(minWidth: 180, maxWidth: 220)

            VStack(spacing: 0) {
                IHMContextHeader(context: .jarvis)

                // ── Header bar ──
                HStack(spacing: 12) {
                    Image(systemName: "sparkles")
                        .foregroundColor(SF.Colors.purple)
                        .font(.system(size: 20))
                    Text(L10n.shared.t(.jarvisTitle))
                        .font(.system(size: 20, weight: .bold))
                        .foregroundColor(SF.Colors.textPrimary)
                    Spacer()
                    HStack(spacing: 6) {
                        StatusDot(active: llm.activeProvider != nil, size: 8)
                        Text(llm.activeDisplayName)
                            .font(.system(size: 11, weight: .medium))
                            .foregroundColor(SF.Colors.textSecondary)
                    }
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(SF.Colors.bgTertiary)
                    .cornerRadius(8)
                }
                .padding(.horizontal, 24)
                .padding(.vertical, 14)
                .background(SF.Colors.bgSecondary)

                Divider().background(SF.Colors.border)

                // ── Chat area ──  Ref: FT-SSF-013
                if chatLoadingState == .loading {
                    SkeletonChatView()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else {
                ScrollViewReader { proxy in
                    ScrollView {
                        VStack(alignment: .leading, spacing: 24) {
                            if messages.isEmpty && !isProcessing && bridge.discussionEvents.isEmpty {
                                emptyState
                            }

                            // Unified message rendering — agent cards or user bubbles
                            ForEach(Array(messages.enumerated()), id: \.offset) { idx, msg in
                                if msg.isAgentMessage {
                                    // ── Phase header (shown before first agent message in a group) ──
                                    if idx == 0 || !messages[idx - 1].isAgentMessage {
                                        phaseHeader
                                    }
                                    agentMessageCardFromStored(msg)
                                } else {
                                    MessageBubble(message: msg)
                                }
                            }

                            // Live thinking/reasoning indicators
                            if bridge.discussionRunning {
                                if bridge.isReasoning {
                                    HStack(spacing: 12) {
                                        ProgressView().controlSize(.small)
                                        Text(L10n.shared.t(.jarvisThinking))
                                            .font(.system(size: 13, weight: .medium))
                                            .foregroundColor(SF.Colors.purple)
                                    }
                                    .padding(.horizontal, 20)
                                    .padding(.vertical, 8)
                                } else {
                                    ForEach(bridge.discussionEvents.filter { $0.eventType == "discuss_thinking" }) { event in
                                        thinkingIndicator(event: event)
                                    }
                                }
                            }

                            if isProcessing && messages.last?.role == "user" && bridge.discussionEvents.isEmpty {
                                HStack(spacing: 12) {
                                    ProgressView().controlSize(.small)
                                    Text(L10n.shared.t(.jarvisTeamMeeting))
                                        .font(.system(size: 14))
                                        .foregroundColor(SF.Colors.textMuted)
                                }
                                .padding(.horizontal, 24)
                            }

                            if let err = errorMessage {
                                HStack(spacing: 8) {
                                    Image(systemName: "exclamationmark.triangle.fill")
                                        .foregroundColor(SF.Colors.error)
                                    Text(err)
                                        .font(.system(size: 13))
                                        .foregroundColor(SF.Colors.error)
                                }
                                .padding(.horizontal, 24)
                            }
                        }
                        .padding(.horizontal, 32)
                        .padding(.vertical, 24)
                        Color.clear.frame(height: 1).id("bottom")
                    }
                    .background(SF.Colors.bgPrimary)
                    .onChange(of: messages.count) { _ in
                        withAnimation(.easeOut(duration: 0.2)) { proxy.scrollTo("bottom", anchor: .bottom) }
                    }
                    .onChange(of: bridge.discussionEvents.count) { _ in
                        // Save new agent events to chat session
                        if let last = bridge.discussionEvents.last, let sid = session?.id {
                            if last.eventType == "discuss_response" {
                                let msg = LLMMessage(
                                    role: "assistant",
                                    content: last.data,
                                    agentId: last.agentId,
                                    agentName: last.agentName.isEmpty ? nil : last.agentName,
                                    agentRole: last.role.isEmpty ? nil : last.role,
                                    messageType: last.messageType,
                                    toAgents: last.toAgents.isEmpty ? nil : last.toAgents
                                )
                                chatStore.appendMessage(msg, to: sid)
                            }
                        }
                        withAnimation(.easeOut(duration: 0.2)) { proxy.scrollTo("bottom", anchor: .bottom) }
                    }
                }
                } // end else (chatLoadingState)  Ref: FT-SSF-013

                Divider().background(SF.Colors.border)

                // ── Input bar ──
                HStack(spacing: 12) {
                    TextField("Message Jarvis…", text: $inputText, axis: .vertical)
                        .lineLimit(1...6)
                        .textFieldStyle(.plain)
                        .font(.system(size: 14))
                        .padding(14)
                        .background(SF.Colors.bgTertiary)
                        .cornerRadius(12)
                        .overlay(
                            RoundedRectangle(cornerRadius: 12)
                                .stroke(SF.Colors.border, lineWidth: 1)
                        )
                        .onSubmit { if !inputText.isEmpty && !isProcessing { Task { await sendMessage() } } }

                    Button(action: { Task { await sendMessage() } }) {
                        Image(systemName: isProcessing ? "stop.circle.fill" : "arrow.up.circle.fill")
                            .font(.system(size: 32))
                            .foregroundColor(isProcessing ? SF.Colors.error : SF.Colors.purple)
                    }
                    .buttonStyle(.plain)
                    .disabled(inputText.isEmpty && !isProcessing)
                }
                .padding(.horizontal, 24)
                .padding(.vertical, 16)
                .background(SF.Colors.bgSecondary)
            }
        }
        .background(SF.Colors.bgPrimary)
        .onAppear {
            if chatStore.activeSession == nil { chatStore.newSession() }
            chatLoadingState = .loaded  // Ref: FT-SSF-013
        }
        .onChange(of: bridge.discussionRunning) { running in
            if !running, let synthesis = bridge.discussionSynthesis {
                Task { @MainActor in
                    processDiscussionResult(synthesis)
                }
            }
        }
    }

    private func newChat() {
        chatStore.newSession()
        bridge.discussionEvents.removeAll()
        errorMessage = nil
    }

    // MARK: - Send message via Rust network discussion

    private func sendMessage() async {
        guard !inputText.isEmpty, !isProcessing else { return }
        guard llm.activeProvider != nil else {
            errorMessage = "Configure an API key in API Keys first."
            return
        }

        let userText = inputText
        inputText = ""
        errorMessage = nil

        let sid = session?.id ?? chatStore.newSession().id
        chatStore.appendMessage(LLMMessage(role: "user", content: userText), to: sid)

        isProcessing = true
        bridge.discussionEvents.removeAll()

        // Build project context for the team
        let projectContext = projects.projects.isEmpty
            ? "No projects exist yet."
            : "Current projects: " + projects.projects.map {
                "\($0.name) (\($0.status.displayName), tech: \($0.tech))"
              }.joined(separator: ", ")

        // Sync LLM config asynchronously (avoids keychain deadlock on main thread)
        await bridge.syncLLMConfigAsync()

        // Trigger Rust network discussion on a background thread to avoid blocking the UI
        bridge.startDiscussionAsync(message: userText, projectContext: projectContext)
    }

    /// Called when the Rust discussion completes — process synthesis and execute actions.
    private func processDiscussionResult(_ synthesis: String) {
        // Execute any actions from the synthesis (CREATE_PROJECT, etc.)
        let actions = JarvisAction.parse(synthesis)
        for action in actions { action.execute() }
        isProcessing = false
    }
}

