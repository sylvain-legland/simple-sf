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
                .frame(minWidth: 180, maxWidth: 220)

            VStack(spacing: 0) {
                // ── Header bar ──
                HStack(spacing: 12) {
                    Image(systemName: "sparkles")
                        .foregroundColor(SF.Colors.purple)
                        .font(.system(size: 20))
                    Text("Jarvis")
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

                // ── Chat area ──
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

                            // Live thinking indicators only
                            if bridge.discussionRunning {
                                ForEach(bridge.discussionEvents.filter { $0.eventType == "discuss_thinking" }) { event in
                                    thinkingIndicator(event: event)
                                }
                            }

                            if isProcessing && messages.last?.role == "user" && bridge.discussionEvents.isEmpty {
                                HStack(spacing: 12) {
                                    ProgressView().controlSize(.small)
                                    Text("L'equipe se reunit...")
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
        }
        .onChange(of: bridge.discussionRunning) { running in
            if !running, let synthesis = bridge.discussionSynthesis {
                Task { @MainActor in
                    processDiscussionResult(synthesis)
                }
            }
        }
    }

    // MARK: - Phase header (shown before first agent message group)

    private var phaseHeader: some View {
        HStack(spacing: 12) {
            Image(systemName: "bubble.left.and.bubble.right.fill")
                .font(.system(size: 18))
                .foregroundColor(SF.Colors.purple)
            Text("Reunion de cadrage")
                .font(.system(size: 17, weight: .semibold))
                .foregroundColor(SF.Colors.textPrimary)
            PatternBadge(pattern: "network")
            Spacer()
            if bridge.discussionRunning {
                HStack(spacing: 6) {
                    ProgressView().controlSize(.small)
                    Text("En cours")
                        .font(.system(size: 12, weight: .medium))
                        .foregroundColor(SF.Colors.textMuted)
                }
            }
        }
        .padding(.horizontal, 20)
        .padding(.vertical, 14)
        .background(SF.Colors.bgTertiary)
        .cornerRadius(12)
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(SF.Colors.border, lineWidth: 0.5)
        )
    }

    // ── Agent message card from stored LLMMessage ──

    @ViewBuilder
    private func agentMessageCardFromStored(_ msg: LLMMessage) -> some View {
        let aid = msg.agentId ?? "engine"
        let name = msg.agentName ?? agentInfo[aid]?.name ?? aid
        let role = msg.agentRole ?? agentInfo[aid]?.role ?? ""
        let mtype = msg.messageType ?? "response"
        let recipients = msg.toAgents ?? []
        let roleColor = roleColorFor(aid)
        let borderColor = borderColorFor(mtype)

        HStack(alignment: .top, spacing: 0) {
            // Left accent border
            RoundedRectangle(cornerRadius: 2)
                .fill(borderColor)
                .frame(width: 4)

            VStack(alignment: .leading, spacing: 12) {
                // ── Metadata header ──
                HStack(spacing: 10) {
                    AgentAvatarView(agentId: aid, size: 44)
                        .overlay(Circle().stroke(roleColor.opacity(0.6), lineWidth: 2.5))

                    VStack(alignment: .leading, spacing: 2) {
                        HStack(spacing: 8) {
                            Text(name)
                                .font(.system(size: 15, weight: .bold))
                                .foregroundColor(roleColor)
                            if mtype != "response" {
                                messageTypeBadge(mtype)
                            }
                        }
                        HStack(spacing: 6) {
                            if !role.isEmpty {
                                Text(role)
                                    .font(.system(size: 12).italic())
                                    .foregroundColor(SF.Colors.textSecondary)
                            }
                            PatternBadge(pattern: "network")
                        }
                    }

                    Spacer()

                    if !recipients.isEmpty {
                        recipientsView(recipients)
                    }
                }

                Divider().background(SF.Colors.border.opacity(0.5))

                // ── Content ──
                MarkdownView(msg.content, fontSize: 14)
                    .textSelection(.enabled)
            }
            .padding(.horizontal, 18)
            .padding(.vertical, 16)
        }
        .background(SF.Colors.bgCard)
        .cornerRadius(12)
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(SF.Colors.border.opacity(0.5), lineWidth: 0.5)
        )
    }

    @ViewBuilder
    private func messageTypeBadge(_ type: String) -> some View {
        let (bg, fg) = badgeColors(type)
        Text(type.uppercased())
            .font(.system(size: 11, weight: .bold))
            .foregroundColor(fg)
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(bg)
            .cornerRadius(6)
    }

    // ── Recipients view (SF legacy: mu__arrow → mu__target) ──

    @ViewBuilder
    private func recipientsView(_ toAgents: [String]) -> some View {
        HStack(spacing: 4) {
            Image(systemName: "arrow.right")
                .font(.system(size: 10, weight: .medium))
                .foregroundColor(SF.Colors.textMuted)
            ForEach(toAgents, id: \.self) { agentId in
                let info = agentInfo[agentId]
                let displayName = info?.name ?? (agentId == "all" ? "Tous" : agentId)
                let color = roleColorFor(agentId)
                HStack(spacing: 4) {
                    AgentAvatarView(agentId: agentId, size: 20)
                    Text(displayName)
                        .font(.system(size: 11, weight: .medium))
                        .foregroundColor(color)
                }
                .padding(.horizontal, 6)
                .padding(.vertical, 3)
                .background(color.opacity(0.1))
                .cornerRadius(6)
            }
        }
    }

    // ── Thinking indicator ──

    @ViewBuilder
    private func thinkingIndicator(event: SFBridge.AgentEvent) -> some View {
        let name = event.agentName.isEmpty
            ? (agentInfo[event.agentId]?.name ?? event.agentId)
            : event.agentName
        HStack(spacing: 12) {
            AgentAvatarView(agentId: event.agentId, size: 32)
            ProgressView().controlSize(.small)
            Text("\(name) redige…")
                .font(.system(size: 13))
                .foregroundColor(SF.Colors.textMuted)
        }
        .padding(.horizontal, 20)
        .padding(.vertical, 8)
    }

    // ── Helper: border color by message type (SF legacy) ──

    private func borderColorFor(_ messageType: String) -> Color {
        switch messageType {
        case "instruction", "request", "delegation":
            return Color(red: 0.92, green: 0.70, blue: 0.03) // yellow #EAB308
        case "response", "approval":
            return Color(red: 0.13, green: 0.77, blue: 0.37) // green #22C55E
        case "veto":
            return Color(red: 0.94, green: 0.27, blue: 0.27) // red #EF4444
        case "synthesis":
            return SF.Colors.po
        default:
            return SF.Colors.textMuted.opacity(0.5) // gray
        }
    }

    // ── Helper: badge colors by message type (SF legacy) ──

    private func badgeColors(_ type: String) -> (Color, Color) {
        switch type {
        case "instruction":
            return (Color(red: 0.92, green: 0.70, blue: 0.03).opacity(0.2), Color(red: 0.92, green: 0.70, blue: 0.03))
        case "delegation":
            return (SF.Colors.purple.opacity(0.2), SF.Colors.purple)
        case "approval":
            return (Color(red: 0.13, green: 0.77, blue: 0.37).opacity(0.2), Color(red: 0.13, green: 0.77, blue: 0.37))
        case "veto":
            return (Color(red: 0.94, green: 0.27, blue: 0.27).opacity(0.2), Color(red: 0.94, green: 0.27, blue: 0.27))
        case "synthesis":
            return (SF.Colors.po.opacity(0.2), SF.Colors.po)
        default:
            return (SF.Colors.textMuted.opacity(0.15), SF.Colors.textMuted)
        }
    }

    // ── Helper: format timestamp as HH:MM ──

    private func formatTime(_ date: Date) -> String {
        let fmt = DateFormatter()
        fmt.dateFormat = "HH:mm"
        return fmt.string(from: date)
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

    // MARK: - Session Sidebar

    private var sessionSidebar: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Historique")
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundColor(SF.Colors.textSecondary)
                Spacer()
                Button(action: { newChat() }) {
                    Image(systemName: "square.and.pencil")
                        .font(.system(size: 14))
                        .foregroundColor(SF.Colors.purple)
                }
                .buttonStyle(.plain)
                .help("Nouvelle conversation")
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 12)

            Divider().background(SF.Colors.border)

            ScrollView {
                LazyVStack(spacing: 2) {
                    ForEach(chatStore.sessions) { sess in
                        Button(action: { chatStore.activeSession = sess }) {
                            HStack {
                                VStack(alignment: .leading, spacing: 3) {
                                    Text(sess.title)
                                        .font(.system(size: 13))
                                        .lineLimit(1)
                                        .foregroundColor(sess.id == session?.id ? SF.Colors.purple : SF.Colors.textPrimary)
                                    Text("\(sess.messages.count) messages")
                                        .font(.system(size: 11))
                                        .foregroundColor(SF.Colors.textMuted)
                                }
                                Spacer()
                            }
                            .padding(.horizontal, 12)
                            .padding(.vertical, 8)
                            .background(sess.id == session?.id ? SF.Colors.purple.opacity(0.1) : Color.clear)
                            .cornerRadius(8)
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(.horizontal, 6)
                .padding(.top, 6)
            }
        }
        .background(SF.Colors.bgSecondary)
    }

    private var emptyState: some View {
        VStack(spacing: 24) {
            HStack(spacing: -10) {
                ForEach(["rte", "architecte", "lead_dev", "product"], id: \.self) { id in
                    AgentAvatarView(agentId: id, size: 56)
                        .overlay(Circle().stroke(SF.Colors.bgPrimary, lineWidth: 3))
                }
            }

            Text("Votre equipe est prete")
                .font(.system(size: 22, weight: .bold))
                .foregroundColor(SF.Colors.textPrimary)
            Text("192 agents  ·  1286 skills  ·  19 patterns\nEssayez : « Fais-moi un Pacman en SwiftUI »")
                .font(.system(size: 14))
                .foregroundColor(SF.Colors.textMuted)
                .multilineTextAlignment(.center)
                .lineSpacing(4)
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 80)
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
        let actions = JarvisAction.parse(synthesis)
        for action in actions { action.execute() }

        let displayText = JarvisAction.cleanDisplay(synthesis)
        if !displayText.isEmpty, let sid = session?.id {
            let msg = LLMMessage(
                role: "assistant",
                content: displayText,
                agentId: "product",
                agentName: agentInfo["product"]?.name,
                agentRole: "Product Owner",
                messageType: "synthesis",
                toAgents: ["all"]
            )
            chatStore.appendMessage(msg, to: sid)
        }

        isProcessing = false
    }
}

struct MessageBubble: View {
    let message: LLMMessage

    var body: some View {
        let isUser = message.role == "user"
        HStack(alignment: .top, spacing: 12) {
            if isUser {
                Spacer(minLength: 80)
            }

            if !isUser {
                Image(systemName: "sparkles")
                    .font(.system(size: 14))
                    .foregroundColor(SF.Colors.purple)
                    .frame(width: 36, height: 36)
                    .background(SF.Colors.purple.opacity(0.12))
                    .clipShape(Circle())
            }

            if isUser {
                Text(message.content)
                    .font(.system(size: 14))
                    .foregroundColor(SF.Colors.textPrimary)
                    .lineSpacing(4)
                    .textSelection(.enabled)
                    .padding(.horizontal, 16)
                    .padding(.vertical, 12)
                    .background(SF.Colors.purple.opacity(0.12))
                    .cornerRadius(16)
                    .overlay(
                        RoundedRectangle(cornerRadius: 16)
                            .stroke(SF.Colors.purple.opacity(0.2), lineWidth: 0.5)
                    )
                    .frame(maxWidth: 700, alignment: .trailing)
            } else {
                MarkdownView(message.content, fontSize: 14)
                    .textSelection(.enabled)
                    .padding(.horizontal, 16)
                    .padding(.vertical, 12)
                    .background(SF.Colors.bgSecondary)
                    .cornerRadius(12)
                    .overlay(
                        RoundedRectangle(cornerRadius: 12)
                            .stroke(SF.Colors.border, lineWidth: 0.5)
                    )
                    .frame(maxWidth: 700, alignment: .leading)
            }

            if isUser {
                Image(systemName: "person.circle.fill")
                    .font(.system(size: 16))
                    .foregroundColor(SF.Colors.textSecondary)
                    .frame(width: 36, height: 36)
            }

            if !isUser {
                Spacer(minLength: 80)
            }
        }
    }
}
