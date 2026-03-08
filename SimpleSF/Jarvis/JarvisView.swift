import SwiftUI

// MARK: - Action parsing (Jarvis manages projects)

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

// MARK: - Agent info for display
private let agentInfo: [String: (name: String, icon: String, color: Color)] = [
    "rte-marie":   ("Marie (RTE)", "person.badge.clock", .blue),
    "po-lucas":    ("Lucas (PO)", "list.clipboard", .green),
    "lead-thomas": ("Thomas (Lead)", "wrench.and.screwdriver", .orange),
    "dev-emma":    ("Emma (Dev)", "laptopcomputer", .pink),
    "dev-karim":   ("Karim (Dev)", "server.rack", .cyan),
    "qa-sophie":   ("Sophie (QA)", "checkmark.shield", .yellow),
    "jarvis":      ("Jarvis", "sparkles", .purple),
    "engine":      ("Système", "gearshape", .gray),
]

// MARK: - JarvisView

@MainActor
struct JarvisView: View {
    @ObservedObject private var llm = LLMService.shared
    @ObservedObject private var bridge = SFBridge.shared
    @ObservedObject private var chatStore = ChatStore.shared
    @ObservedObject private var projects = ProjectStore.shared

    @State private var inputText = ""
    @State private var isProcessing = false
    @State private var errorMessage: String?

    private var session: ChatSession? { chatStore.activeSession }
    private var messages: [LLMMessage] { session?.messages ?? [] }

    var body: some View {
        HSplitView {
            sessionSidebar
                .frame(minWidth: 180, maxWidth: 220)

            VStack(spacing: 0) {
                // Header
                HStack {
                    Image(systemName: "sparkles")
                        .foregroundColor(.purple)
                    Text("Jarvis")
                        .font(.title2.bold())
                    Spacer()
                    if let prov = llm.activeProvider {
                        Label(prov.displayName, systemImage: "checkmark.circle.fill")
                            .font(.caption)
                            .foregroundColor(.green)
                    } else {
                        Label("No provider", systemImage: "exclamationmark.circle")
                            .font(.caption)
                            .foregroundColor(.orange)
                    }
                }
                .padding()

                Divider()

                // Messages + discussion thread
                ScrollViewReader { proxy in
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 12) {
                            if messages.isEmpty && !isProcessing && bridge.discussionEvents.isEmpty {
                                emptyState
                            }

                            ForEach(Array(messages.enumerated()), id: \.offset) { _, msg in
                                MessageRow(message: msg)
                            }

                            // Live discussion thread from Rust engine
                            if !bridge.discussionEvents.isEmpty {
                                discussionThread
                            }

                            if isProcessing && bridge.discussionEvents.isEmpty {
                                HStack(spacing: 8) {
                                    ProgressView().controlSize(.small)
                                    Text("L'équipe discute...")
                                        .font(.callout)
                                        .foregroundColor(.secondary)
                                }
                                .padding(.horizontal)
                            }

                            if let err = errorMessage {
                                Text(err)
                                    .foregroundColor(.red)
                                    .font(.caption)
                                    .padding(.horizontal)
                            }
                        }
                        .padding()
                        Color.clear.frame(height: 1).id("bottom")
                    }
                    .onChange(of: bridge.discussionEvents.count) { _ in
                        withAnimation { proxy.scrollTo("bottom", anchor: .bottom) }
                    }
                    .onChange(of: messages.count) { _ in
                        withAnimation { proxy.scrollTo("bottom", anchor: .bottom) }
                    }
                }

                Divider()

                // Input
                HStack(spacing: 8) {
                    TextField("Message Jarvis…", text: $inputText, axis: .vertical)
                        .lineLimit(1...6)
                        .textFieldStyle(.plain)
                        .padding(10)
                        .background(Color(.controlBackgroundColor))
                        .cornerRadius(8)
                        .onSubmit { if !inputText.isEmpty && !isProcessing { Task { await sendMessage() } } }

                    Button(action: { Task { await sendMessage() } }) {
                        Image(systemName: isProcessing ? "stop.circle.fill" : "arrow.up.circle.fill")
                            .font(.system(size: 28))
                            .foregroundColor(isProcessing ? .red : .purple)
                    }
                    .buttonStyle(.plain)
                    .disabled(inputText.isEmpty && !isProcessing)
                }
                .padding()
            }
        }
        .onAppear {
            if chatStore.activeSession == nil { chatStore.newSession() }
        }
        // Watch for discussion completion → process synthesis
        .onChange(of: bridge.discussionRunning) { running in
            if !running, let synthesis = bridge.discussionSynthesis {
                Task { @MainActor in
                    processDiscussionResult(synthesis)
                }
            }
        }
    }

    // MARK: - Discussion Thread (shows each agent's contribution)

    private var discussionThread: some View {
        VStack(alignment: .leading, spacing: 8) {
            ForEach(bridge.discussionEvents) { event in
                if event.eventType == "discuss_response" {
                    let info = agentInfo[event.agentId] ?? (event.agentId, "person.circle", .gray)
                    HStack(alignment: .top, spacing: 10) {
                        Image(systemName: info.icon)
                            .foregroundColor(info.color)
                            .frame(width: 20)
                        VStack(alignment: .leading, spacing: 4) {
                            Text(info.name)
                                .font(.caption.bold())
                                .foregroundColor(info.color)
                            Text(event.data)
                                .textSelection(.enabled)
                                .frame(maxWidth: .infinity, alignment: .leading)
                        }
                    }
                    .padding(.vertical, 4)
                    .padding(.horizontal, 8)
                    .background(info.color.opacity(0.05))
                    .cornerRadius(8)
                } else if event.eventType == "discuss_thinking" {
                    let info = agentInfo[event.agentId] ?? (event.agentId, "person.circle", .gray)
                    HStack(spacing: 8) {
                        ProgressView().controlSize(.mini)
                        Text("\(info.name) réfléchit…")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }

            if bridge.discussionRunning {
                HStack(spacing: 8) {
                    ProgressView().controlSize(.small)
                    Text("Discussion en cours…")
                        .font(.callout)
                        .foregroundColor(.secondary)
                }
            }
        }
    }

    // MARK: - Session Sidebar

    private var sessionSidebar: some View {
        VStack(spacing: 0) {
            HStack {
                Text("History")
                    .font(.headline)
                    .foregroundColor(.secondary)
                Spacer()
                Button(action: { newChat() }) {
                    Image(systemName: "square.and.pencil")
                        .font(.callout)
                        .foregroundColor(.purple)
                }
                .buttonStyle(.plain)
                .help("New conversation")
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 10)

            Divider()

            ScrollView {
                LazyVStack(spacing: 2) {
                    ForEach(chatStore.sessions) { sess in
                        Button(action: { chatStore.activeSession = sess }) {
                            HStack {
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(sess.title)
                                        .font(.callout)
                                        .lineLimit(1)
                                        .foregroundColor(sess.id == session?.id ? .purple : .primary)
                                    Text("\(sess.messages.count) messages")
                                        .font(.caption2)
                                        .foregroundColor(.secondary)
                                }
                                Spacer()
                            }
                            .padding(.horizontal, 12)
                            .padding(.vertical, 8)
                            .background(sess.id == session?.id ? Color.purple.opacity(0.1) : Color.clear)
                            .cornerRadius(8)
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(.horizontal, 4)
                .padding(.top, 4)
            }
        }
        .background(Color(.controlBackgroundColor).opacity(0.5))
    }

    private var emptyState: some View {
        VStack(spacing: 16) {
            Image(systemName: "sparkles")
                .font(.system(size: 48))
                .foregroundColor(.purple.opacity(0.4))
            Text("Ask Jarvis anything")
                .font(.title3)
                .foregroundColor(.secondary)
            Text("Your team of 6 AI agents will discuss and execute your requests.\nTry: \"Create a web app called HelloWorld\"")
                .font(.caption)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 60)
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

        // Trigger Rust network discussion (RTE + PO + Jarvis discuss)
        bridge.syncLLMConfig()
        let _ = bridge.startDiscussion(message: userText, projectContext: projectContext)

        // The result is handled in .onChange(of: bridge.discussionRunning)
    }

    /// Called when the Rust discussion completes — process synthesis and execute actions.
    private func processDiscussionResult(_ synthesis: String) {
        let sid = session?.id ?? chatStore.newSession().id

        // Build a summary of the discussion for the chat history
        let discussionSummary = bridge.discussionEvents
            .filter { $0.eventType == "discuss_response" }
            .map { event -> String in
                let info = agentInfo[event.agentId] ?? (event.agentId, "", .gray)
                return "**\(info.name)**: \(event.data)"
            }
            .joined(separator: "\n\n---\n\n")

        if !discussionSummary.isEmpty {
            chatStore.appendMessage(
                LLMMessage(role: "assistant", content: discussionSummary),
                to: sid
            )
        }

        // Execute any actions from the synthesis (CREATE_PROJECT, START_MISSION, etc.)
        let actions = JarvisAction.parse(synthesis)
        for action in actions { action.execute() }

        // Show Jarvis's final synthesis (cleaned of action tags)
        let displayText = JarvisAction.cleanDisplay(synthesis)
        if !displayText.isEmpty {
            chatStore.appendMessage(
                LLMMessage(role: "assistant", content: "🎯 **Jarvis**: \(displayText)"),
                to: sid
            )
        }

        bridge.discussionEvents.removeAll()
        isProcessing = false
    }
}

struct MessageRow: View {
    let message: LLMMessage
    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: message.role == "user" ? "person.circle" : "sparkles")
                .foregroundColor(message.role == "user" ? .blue : .purple)
                .frame(width: 20)
            Text(message.content)
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.vertical, 4)
    }
}
