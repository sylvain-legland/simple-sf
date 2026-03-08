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
private let agentInfo: [String: (name: String, role: String, icon: String, color: Color)] = [
    // Intake team (matching SF platform DB IDs)
    "rte":            ("Marc Delacroix",    "RTE",           "person.badge.clock",        .blue),
    "product":        ("Laura Vidal",       "Product Owner", "list.clipboard",            .green),
    "architecte":     ("Pierre Duval",      "Architecte",    "building.2",                .indigo),
    "lead_dev":       ("Thomas Dubois",     "Lead Dev",      "wrench.and.screwdriver",    .orange),
    // Dev team
    "dev":            ("Maxime Bernard",    "Developer",     "laptopcomputer",            .cyan),
    "dev_frontend":   ("Emma Laurent",      "Dev Frontend",  "paintbrush",                .pink),
    "dev_backend":    ("Julien Moreau",     "Dev Backend",   "server.rack",               .mint),
    "dev_fullstack":  ("Alex Petit",        "Dev Fullstack", "macbook.and.iphone",        .purple),
    "dev_mobile":     ("Romain Faure",      "Dev Mobile",    "iphone",                    .orange),
    // QA & Ops
    "qa_lead":        ("Claire Rousseau",   "QA Lead",       "checkmark.shield",          .yellow),
    "tester":         ("Éric Fontaine",     "QA",            "checklist",                 .yellow),
    "devops":         ("Karim Diallo",      "DevOps",        "cloud",                     .blue),
    "securite":       ("Marc Lefranc",      "Sécurité",      "lock.shield",               .red),
    "ux_designer":    ("Chloé Bernard",     "UX Designer",   "paintpalette",              .pink),
    "data_engineer":  ("Antoine Roux",      "Data Engineer", "chart.bar",                 .green),
    "tech_writer":    ("Valérie Morin",     "Tech Writer",   "doc.text",                  .gray),
    "cloud_architect":("Romain Vasseur",    "Cloud Archi",   "cloud.bolt",                .blue),
    // Strategic
    "strat-cto":      ("Karim Benali",      "CTO",           "gear.badge.checkmark",      .purple),
    "strat-cpo":      ("Julie Marchand",    "CPO",           "star.circle",               .orange),
    "brain":          ("Gabriel Mercier",   "Orchestrateur", "brain.head.profile",        .purple),
    // System
    "jarvis":         ("Jarvis",            "Chef de projet","sparkles",                  .purple),
    "engine":         ("Système",           "",              "gearshape",                 .gray),
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
                .frame(minWidth: 170, maxWidth: 210)

            VStack(spacing: 0) {
                // Header
                HStack(spacing: 8) {
                    Image(systemName: "sparkles")
                        .foregroundColor(SF.Colors.purple)
                        .font(.title3)
                    Text("Jarvis")
                        .font(SF.Font.title)
                        .foregroundColor(SF.Colors.textPrimary)
                    Spacer()
                    // Provider badge
                    HStack(spacing: 4) {
                        StatusDot(active: llm.activeProvider != nil)
                        Text(llm.activeDisplayName)
                            .font(SF.Font.caption)
                            .foregroundColor(SF.Colors.textSecondary)
                    }
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(SF.Colors.bgTertiary)
                    .cornerRadius(SF.Radius.md)
                }
                .padding(.horizontal, SF.Spacing.lg)
                .padding(.vertical, SF.Spacing.md)

                Divider().background(SF.Colors.border)

                // Messages + discussion thread
                ScrollViewReader { proxy in
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: SF.Spacing.md) {
                            if messages.isEmpty && !isProcessing && bridge.discussionEvents.isEmpty {
                                emptyState
                            }

                            ForEach(Array(messages.enumerated()), id: \.offset) { _, msg in
                                MessageBubble(message: msg)
                            }

                            // Live discussion thread from Rust engine
                            if !bridge.discussionEvents.isEmpty {
                                discussionThread
                            }

                            if isProcessing && bridge.discussionEvents.isEmpty {
                                HStack(spacing: 8) {
                                    ProgressView().controlSize(.small)
                                    Text("L'équipe se réunit…")
                                        .font(SF.Font.body)
                                        .foregroundColor(SF.Colors.textMuted)
                                }
                                .padding(.horizontal, SF.Spacing.lg)
                            }

                            if let err = errorMessage {
                                HStack(spacing: 6) {
                                    Image(systemName: "exclamationmark.triangle.fill")
                                        .foregroundColor(SF.Colors.error)
                                    Text(err)
                                        .font(SF.Font.caption)
                                        .foregroundColor(SF.Colors.error)
                                }
                                .padding(.horizontal, SF.Spacing.lg)
                            }
                        }
                        .padding(SF.Spacing.lg)
                        Color.clear.frame(height: 1).id("bottom")
                    }
                    .onChange(of: bridge.discussionEvents.count) { _ in
                        withAnimation(.easeOut(duration: 0.2)) { proxy.scrollTo("bottom", anchor: .bottom) }
                    }
                    .onChange(of: messages.count) { _ in
                        withAnimation(.easeOut(duration: 0.2)) { proxy.scrollTo("bottom", anchor: .bottom) }
                    }
                }

                Divider().background(SF.Colors.border)

                // Input
                HStack(spacing: 8) {
                    TextField("Message Jarvis…", text: $inputText, axis: .vertical)
                        .lineLimit(1...6)
                        .textFieldStyle(.plain)
                        .font(SF.Font.body)
                        .padding(SF.Spacing.md)
                        .background(SF.Colors.bgTertiary)
                        .cornerRadius(SF.Radius.md)
                        .overlay(
                            RoundedRectangle(cornerRadius: SF.Radius.md)
                                .stroke(SF.Colors.border, lineWidth: 1)
                        )
                        .onSubmit { if !inputText.isEmpty && !isProcessing { Task { await sendMessage() } } }

                    Button(action: { Task { await sendMessage() } }) {
                        Image(systemName: isProcessing ? "stop.circle.fill" : "arrow.up.circle.fill")
                            .font(.system(size: 28))
                            .foregroundColor(isProcessing ? SF.Colors.error : SF.Colors.purple)
                    }
                    .buttonStyle(.plain)
                    .disabled(inputText.isEmpty && !isProcessing)
                }
                .padding(SF.Spacing.lg)
            }
            .background(SF.Colors.bgPrimary)
        }
        .onAppear {
            if chatStore.activeSession == nil { chatStore.newSession() }
        }
        .onChange(of: bridge.discussionRunning) { running in
            if !running, let synthesis = bridge.discussionSynthesis {
                Task { @MainActor in
                    processDiscussionResult(synthesis)
                }
            }
        }
    }

    // MARK: - Discussion Thread

    private var discussionThread: some View {
        VStack(alignment: .leading, spacing: 2) {
            // Phase/pattern header
            HStack(spacing: 6) {
                Image(systemName: "bubble.left.and.bubble.right.fill")
                    .font(.caption)
                    .foregroundColor(SF.Colors.purple)
                Text("Réunion de cadrage")
                    .font(SF.Font.headline)
                    .foregroundColor(SF.Colors.textPrimary)
                PatternBadge(pattern: "network")
                Spacer()
                if bridge.discussionRunning {
                    ProgressView().controlSize(.mini)
                }
            }
            .padding(.bottom, SF.Spacing.sm)

            // Participant pills
            HStack(spacing: -6) {
                ForEach(["rte", "architecte", "lead_dev", "product"], id: \.self) { id in
                    AgentAvatarView(agentId: id, size: 24)
                }
            }
            .padding(.bottom, SF.Spacing.md)

            // Agent messages
            ForEach(bridge.discussionEvents) { event in
                if event.eventType == "discuss_response" {
                    let info = agentInfo[event.agentId] ?? (event.agentId, "", "person.circle", .gray)
                    let roleColor = roleColorFor(event.agentId)

                    VStack(alignment: .leading, spacing: 0) {
                        // Agent header
                        HStack(spacing: SF.Spacing.sm) {
                            AgentAvatarView(agentId: event.agentId, size: 32)
                            VStack(alignment: .leading, spacing: 1) {
                                Text(info.name)
                                    .font(SF.Font.headline)
                                    .foregroundColor(SF.Colors.textPrimary)
                                RoleBadge(role: info.role, color: roleColor)
                            }
                            Spacer()
                            // Recipients
                            Text(recipientsFor(event.agentId))
                                .font(SF.Font.badge)
                                .foregroundColor(SF.Colors.textMuted)
                        }
                        .padding(.bottom, SF.Spacing.xs)

                        // Message content
                        Text(event.data)
                            .font(SF.Font.body)
                            .foregroundColor(SF.Colors.textPrimary)
                            .textSelection(.enabled)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(SF.Spacing.md)
                            .background(SF.Colors.bgSecondary)
                            .cornerRadius(SF.Radius.md)
                            .overlay(
                                RoundedRectangle(cornerRadius: SF.Radius.md)
                                    .stroke(roleColor.opacity(0.15), lineWidth: 1)
                            )
                    }
                    .padding(.vertical, SF.Spacing.xs)

                } else if event.eventType == "discuss_thinking" {
                    let info = agentInfo[event.agentId] ?? (event.agentId, "", "person.circle", .gray)
                    HStack(spacing: SF.Spacing.sm) {
                        AgentAvatarView(agentId: event.agentId, size: 24)
                        ProgressView().controlSize(.mini)
                        Text("\(info.name) réfléchit…")
                            .font(SF.Font.caption)
                            .foregroundColor(SF.Colors.textMuted)
                    }
                    .padding(.vertical, SF.Spacing.xs)
                }
            }
        }
        .padding(SF.Spacing.md)
        .background(SF.Colors.bgCard.opacity(0.5))
        .cornerRadius(SF.Radius.lg)
        .overlay(
            RoundedRectangle(cornerRadius: SF.Radius.lg)
                .stroke(SF.Colors.border, lineWidth: 1)
        )
    }

    /// Role-based color for agent
    private func roleColorFor(_ agentId: String) -> Color {
        switch agentId {
        case "rte":          return SF.Colors.rte
        case "product":      return SF.Colors.po
        case "architecte":   return SF.Colors.architect
        case "lead_dev":     return SF.Colors.lead
        default:
            let info = agentInfo[agentId]
            return info?.color ?? SF.Colors.textMuted
        }
    }

    /// Recipients display: "@Pierre @Thomas @Laura"
    private func recipientsFor(_ senderId: String) -> String {
        let all = ["rte", "architecte", "lead_dev", "product"]
        let others = all.filter { $0 != senderId }
        return others.compactMap { agentInfo[$0]?.name.split(separator: " ").first.map { "@\($0)" } }.joined(separator: " ")
    }

    // MARK: - Session Sidebar

    private var sessionSidebar: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Historique")
                    .font(SF.Font.headline)
                    .foregroundColor(SF.Colors.textSecondary)
                Spacer()
                Button(action: { newChat() }) {
                    Image(systemName: "square.and.pencil")
                        .font(.callout)
                        .foregroundColor(SF.Colors.purple)
                }
                .buttonStyle(.plain)
                .help("Nouvelle conversation")
            }
            .padding(.horizontal, SF.Spacing.md)
            .padding(.vertical, SF.Spacing.md)

            Divider().background(SF.Colors.border)

            ScrollView {
                LazyVStack(spacing: 2) {
                    ForEach(chatStore.sessions) { sess in
                        Button(action: { chatStore.activeSession = sess }) {
                            HStack {
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(sess.title)
                                        .font(SF.Font.body)
                                        .lineLimit(1)
                                        .foregroundColor(sess.id == session?.id ? SF.Colors.purple : SF.Colors.textPrimary)
                                    Text("\(sess.messages.count) messages")
                                        .font(SF.Font.caption)
                                        .foregroundColor(SF.Colors.textMuted)
                                }
                                Spacer()
                            }
                            .padding(.horizontal, SF.Spacing.md)
                            .padding(.vertical, SF.Spacing.sm)
                            .background(sess.id == session?.id ? SF.Colors.purple.opacity(0.1) : Color.clear)
                            .cornerRadius(SF.Radius.md)
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(.horizontal, SF.Spacing.xs)
                .padding(.top, SF.Spacing.xs)
            }
        }
        .background(SF.Colors.bgSecondary)
    }

    private var emptyState: some View {
        VStack(spacing: SF.Spacing.lg) {
            // Team avatars
            HStack(spacing: -8) {
                ForEach(["rte", "architecte", "lead_dev", "product"], id: \.self) { id in
                    AgentAvatarView(agentId: id, size: 40)
                }
            }

            Text("Votre équipe est prête")
                .font(SF.Font.title)
                .foregroundColor(SF.Colors.textPrimary)
            Text("192 agents · 1286 skills · 19 patterns\nEssayez : « Fais-moi un Pacman en SwiftUI »")
                .font(SF.Font.body)
                .foregroundColor(SF.Colors.textMuted)
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
                let info = agentInfo[event.agentId] ?? (event.agentId, "", "", .gray)
                let roleTag = info.role.isEmpty ? "" : " (\(info.role))"
                return "**\(info.name)\(roleTag)**: \(event.data)"
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

        // Show PO's final synthesis (cleaned of action tags)
        let displayText = JarvisAction.cleanDisplay(synthesis)
        if !displayText.isEmpty {
            let poInfo = agentInfo["product"]!
            chatStore.appendMessage(
                LLMMessage(role: "assistant", content: "📋 **\(poInfo.name) (\(poInfo.role))**: \(displayText)"),
                to: sid
            )
        }

        bridge.discussionEvents.removeAll()
        isProcessing = false
    }
}

struct MessageBubble: View {
    let message: LLMMessage

    var body: some View {
        let isUser = message.role == "user"
        HStack(alignment: .top, spacing: SF.Spacing.md) {
            if isUser {
                Spacer(minLength: 60)
            }

            if !isUser {
                Image(systemName: "sparkles")
                    .foregroundColor(SF.Colors.purple)
                    .frame(width: 24, height: 24)
                    .background(SF.Colors.purple.opacity(0.12))
                    .clipShape(Circle())
            }

            Text(message.content)
                .font(SF.Font.body)
                .foregroundColor(SF.Colors.textPrimary)
                .textSelection(.enabled)
                .padding(SF.Spacing.md)
                .background(isUser ? SF.Colors.purple.opacity(0.12) : SF.Colors.bgSecondary)
                .cornerRadius(SF.Radius.lg)
                .overlay(
                    RoundedRectangle(cornerRadius: SF.Radius.lg)
                        .stroke(isUser ? SF.Colors.purple.opacity(0.2) : SF.Colors.border, lineWidth: 0.5)
                )
                .frame(maxWidth: .infinity, alignment: isUser ? .trailing : .leading)

            if isUser {
                Image(systemName: "person.circle.fill")
                    .foregroundColor(SF.Colors.textSecondary)
                    .frame(width: 24, height: 24)
            }

            if !isUser {
                Spacer(minLength: 60)
            }
        }
    }
}
