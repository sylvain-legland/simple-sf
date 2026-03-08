import SwiftUI

// MARK: - Action parsing (Jarvis manages projects)

enum JarvisAction {
    case createProject(name: String, description: String, tech: String)
    case deleteProject(name: String)
    case updateProject(name: String, status: ProjectStatus?)
    case startMission(projectName: String, brief: String)

    static func parse(_ text: String) -> [JarvisAction] {
        var actions: [JarvisAction] = []

        // Extract name/description/tech from tag content
        func attr(_ key: String, in block: String) -> String? {
            guard let range = block.range(of: "\(key)=\"") else { return nil }
            let start = range.upperBound
            guard let end = block[start...].firstIndex(of: "\"") else { return nil }
            return String(block[start..<end])
        }

        // Find all [TAG ...] blocks
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
            // Also create in Rust engine
            let _ = bridge.createProject(name: name, description: desc, tech: tech)
        case .deleteProject(let name):
            if let p = store.projects.first(where: { $0.name.lowercased() == name.lowercased() }) {
                store.delete(p.id)
            }
            // Also delete from Rust engine
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
            // Find project in Rust engine, or create one
            var rustProjects = bridge.listProjects()
            var projectId = rustProjects.first(where: { $0.name.lowercased() == projectName.lowercased() })?.id
            if projectId == nil {
                // Auto-create from Swift store if it exists there
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

    // Strip action tags from displayed text
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

// MARK: - System prompt with project tools

private let jarvisSystemPrompt = """
You are Jarvis, a helpful software engineering AI assistant embedded in a native macOS app.
Be concise and precise. You manage the user's projects and can launch SAFe missions.

You have a full Software Factory engine embedded locally with a team of 6 agents:
- Marie Lefevre (RTE) — coordinates the team
- Lucas Martin (PO) — defines user stories and acceptance criteria
- Thomas Dubois (Lead Dev) — architecture and code review
- Emma Laurent (Frontend Dev) — UI implementation
- Karim Benali (Backend Dev) — backend implementation
- Sophie Durand (QA) — testing and quality

AVAILABLE ACTIONS (include these tags in your response when needed):

To create a project:
[CREATE_PROJECT name="Project Name" description="Short description" tech="Swift, Python, etc."]

To delete a project:
[DELETE_PROJECT name="Project Name"]

To update a project status (idea, planning, active, paused, done):
[UPDATE_PROJECT name="Project Name" status="active"]

To start a SAFe mission (triggers the full agent team workflow):
[START_MISSION project="Project Name" brief="Detailed description of what to build"]

RULES:
- When the user says "create a project", ALWAYS include a [CREATE_PROJECT ...] tag
- When the user says "delete/remove a project", ALWAYS include a [DELETE_PROJECT ...] tag
- When the user asks to BUILD, CODE, or DEVELOP something, ALWAYS:
  1. Create the project first with [CREATE_PROJECT ...] if it doesn't exist
  2. Then start a mission with [START_MISSION ...] with a detailed brief
- The mission will trigger the SAFe workflow: Vision → Design → Development → QA → Review
- The action tags will be hidden from the user, they only see your text
- After starting a mission, tell the user to check the Missions tab to see progress
"""

// MARK: - JarvisView

@MainActor
struct JarvisView: View {
    @ObservedObject private var llm = LLMService.shared
    @ObservedObject private var chatStore = ChatStore.shared
    @ObservedObject private var projects = ProjectStore.shared

    @State private var inputText = ""
    @State private var isStreaming = false
    @State private var streamingContent = ""
    @State private var errorMessage: String?

    private var session: ChatSession? { chatStore.activeSession }
    private var messages: [LLMMessage] { session?.messages ?? [] }

    var body: some View {
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

            // Messages
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 12) {
                        if messages.isEmpty && !isStreaming {
                            emptyState
                        }
                        ForEach(Array(messages.enumerated()), id: \.offset) { _, msg in
                            MessageRow(message: msg)
                        }
                        if isStreaming {
                            MessageRow(message: LLMMessage(
                                role: "assistant",
                                content: JarvisAction.cleanDisplay(streamingContent) + "▊"
                            ))
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
                .onChange(of: streamingContent) { _ in
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
                    .onSubmit { if !inputText.isEmpty && !isStreaming { Task { await sendMessage() } } }

                Button(action: { Task { await sendMessage() } }) {
                    Image(systemName: isStreaming ? "stop.circle.fill" : "arrow.up.circle.fill")
                        .font(.system(size: 28))
                        .foregroundColor(isStreaming ? .red : .purple)
                }
                .buttonStyle(.plain)
                .disabled(inputText.isEmpty && !isStreaming)
            }
            .padding()
        }
        .onAppear {
            if chatStore.activeSession == nil { chatStore.newSession() }
        }
    }

    private var emptyState: some View {
        VStack(spacing: 16) {
            Image(systemName: "sparkles")
                .font(.system(size: 48))
                .foregroundColor(.purple.opacity(0.4))
            Text("Ask Jarvis anything")
                .font(.title3)
                .foregroundColor(.secondary)
            Text("Your local AI assistant. Manages your projects too.\nTry: \"Create a project called MyApp using SwiftUI\"")
                .font(.caption)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 60)
    }

    private func newChat() {
        chatStore.newSession()
        errorMessage = nil
    }

    private func sendMessage() async {
        guard !inputText.isEmpty, !isStreaming else { return }
        guard llm.activeProvider != nil else {
            errorMessage = "Configure an API key in API Keys first."
            return
        }

        let userText = inputText
        inputText = ""
        errorMessage = nil

        let sid = session?.id ?? chatStore.newSession().id
        chatStore.appendMessage(LLMMessage(role: "user", content: userText), to: sid)

        isStreaming = true
        streamingContent = ""

        // Build context with current projects
        let projectContext = projects.projects.isEmpty
            ? "No projects exist yet."
            : "Current projects: " + projects.projects.map { "\($0.name) (\($0.status.displayName), tech: \($0.tech))" }.joined(separator: ", ")

        let langName = OnboardingView.languages.first(where: { $0.code == AppState.shared.selectedLang })?.name ?? "English"
        let fullSystem = jarvisSystemPrompt + "\n\nCONTEXT:\n\(projectContext)\n\nIMPORTANT: Always respond in \(langName)."

        let history = chatStore.sessions.first(where: { $0.id == sid })?.messages ?? []
        let stream = llm.stream(messages: history, system: fullSystem)

        for await chunk in stream {
            streamingContent += chunk
        }

        // Execute any actions in the response
        let actions = JarvisAction.parse(streamingContent)
        for action in actions { action.execute() }

        // Store cleaned message (without action tags)
        let displayText = JarvisAction.cleanDisplay(streamingContent)
        if !displayText.isEmpty {
            chatStore.appendMessage(LLMMessage(role: "assistant", content: displayText), to: sid)
        }
        streamingContent = ""
        isStreaming = false
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
