import SwiftUI

// MARK: - Pilot Projects (AC validation — matches SF legacy)

private let pilotProjects: [(name: String, tech: String, description: String)] = [
    ("Design System WCAG AA",
     "React, TypeScript, CSS",
     "30+ composants React TypeScript accessibles, design tokens CSS, documentation Storybook"),
    ("SDK Mobile Universel",
     "React Native, Expo, TypeScript",
     "Composants UI cross-platform, auth biometric, navigation, state management"),
    ("Plateforme ML Distribuée",
     "Python, PyTorch, FastAPI",
     "Training multi-worker, hyperparameter tuning, model registry, serving API REST"),
    ("Orchestrateur Workflows Data",
     "Python, Airflow, React",
     "DAG visual, scheduling CRON, retry logic, monitoring temps réel, connecteurs"),
    ("Marketplace SaaS Multi-Tenant",
     "Node.js, Next.js, PostgreSQL",
     "Multi-tenancy, billing Stripe, RBAC, API REST + GraphQL, dashboard analytics"),
    ("Migration Angular → React",
     "TypeScript, Angular, React",
     "Migration progressive 50+ composants Angular 14 vers React 18 avec feature parity"),
    ("Jeu Pac-Man macOS Natif",
     "Swift, SwiftUI, SpriteKit",
     "Pac-Man native macOS avec SpriteKit, niveaux, IA fantômes, scores persistants"),
    ("API Gateway Rust",
     "Rust, Tokio, gRPC",
     "Reverse proxy haute performance, rate limiting, auth JWT, observabilité OpenTelemetry"),
]

// MARK: - 14-phase SAFe product lifecycle (matches SF legacy Value Stream)

private let safePhases: [(name: String, short: String, pattern: String)] = [
    ("Idéation",            "Idéation",       "network"),
    ("Comité Stratégique",  "Comité Strat.",   "human-in-the-loop"),
    ("Constitution",        "Constitution",    "sequential"),
    ("Architecture",        "Architecture",    "aggregator"),
    ("Design System",       "Design Sys.",     "sequential"),
    ("Sprints Dev",         "Sprints Dev",     "hierarchical"),
    ("Build & Verify",      "Build & Verify",  "sequential"),
    ("Pipeline CI/CD",      "Pipeline CI",     "sequential"),
    ("Revue UX",            "Revue UX",        "loop"),
    ("Campagne QA",         "Campagne QA",     "loop"),
    ("Exécution Tests",     "Exécution",       "parallel"),
    ("Deploy Prod",         "Deploy Prod",     "human-in-the-loop"),
    ("Routage TMA",         "Routage",         "router"),
    ("Correctif & TMA",     "Correctif",       "loop"),
]

private func simulatedActivePhase(for status: ProjectStatus, progress: Double) -> Int {
    switch status {
    case .idea:     return 0
    case .planning: return 1
    case .active:   return max(2, min(13, Int(progress * 13.0)))
    case .paused:   return max(2, min(13, Int(progress * 13.0)))
    case .done:     return 14
    }
}

// MARK: - Projects View (accordion: card + inline discussion)

@MainActor
struct ProjectsView: View {
    @ObservedObject private var store = ProjectStore.shared
    @ObservedObject private var bridge = SFBridge.shared
    @State private var searchText = ""
    @State private var expandedProjectId: String?

    private var filtered: [Project] {
        guard !searchText.isEmpty else { return store.projects }
        return store.projects.filter {
            $0.name.localizedCaseInsensitiveContains(searchText) ||
            $0.description.localizedCaseInsensitiveContains(searchText)
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack(spacing: 10) {
                Image(systemName: "folder.fill")
                    .font(.system(size: 20))
                    .foregroundColor(SF.Colors.purple)
                Text("Projects")
                    .font(.system(size: 22, weight: .bold))
                    .foregroundColor(SF.Colors.textPrimary)
                Spacer()
                Text("\(store.projects.count) projects")
                    .font(.system(size: 12))
                    .foregroundColor(SF.Colors.textSecondary)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 4)
                    .background(SF.Colors.bgTertiary)
                    .cornerRadius(6)
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 16)

            if !store.projects.isEmpty {
                HStack(spacing: 8) {
                    Image(systemName: "magnifyingglass")
                        .font(.system(size: 12))
                        .foregroundColor(SF.Colors.textMuted)
                    TextField("Search projects…", text: $searchText)
                        .textFieldStyle(.plain)
                        .font(.system(size: 13))
                        .foregroundColor(SF.Colors.textPrimary)
                }
                .padding(10)
                .background(SF.Colors.bgTertiary)
                .cornerRadius(SF.Radius.md)
                .overlay(RoundedRectangle(cornerRadius: SF.Radius.md).stroke(SF.Colors.border, lineWidth: 1))
                .padding(.horizontal, 24)
            }

            Divider().background(SF.Colors.border).padding(.top, 10)

            ScrollView {
                LazyVStack(spacing: 12) {
                    if !store.projects.isEmpty {
                        ForEach(filtered) { project in
                            ProjectAccordion(
                                project: project,
                                isExpanded: expandedProjectId == project.id,
                                toggle: { toggleExpand(project.id) }
                            )
                        }
                    } else {
                        emptyState
                    }
                }
                .padding(24)

                pilotSection
            }
        }
        .background(SF.Colors.bgPrimary)
    }

    private func toggleExpand(_ id: String) {
        withAnimation(.easeInOut(duration: 0.25)) {
            expandedProjectId = expandedProjectId == id ? nil : id
        }
    }

    // MARK: - Pilot Projects Section

    private var pilotSection: some View {
        VStack(spacing: 0) {
            Divider().background(SF.Colors.border)

            HStack(spacing: 10) {
                Image(systemName: "flag.fill")
                    .font(.system(size: 14))
                    .foregroundColor(SF.Colors.accent)
                Text("Projets Pilotes")
                    .font(.system(size: 16, weight: .bold))
                    .foregroundColor(SF.Colors.textPrimary)
                Text("AC validation")
                    .font(.system(size: 11))
                    .foregroundColor(SF.Colors.textMuted)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 2)
                    .background(SF.Colors.accent.opacity(0.15))
                    .cornerRadius(4)
                Spacer()
                Button {
                    loadPilotProjects()
                } label: {
                    Label("Charger", systemImage: "plus.circle.fill")
                        .font(.system(size: 12, weight: .medium))
                        .foregroundColor(SF.Colors.purple)
                }
                .buttonStyle(.plain)

                Button {
                    resetPilotProjects()
                } label: {
                    Label("Réinitialiser", systemImage: "arrow.counterclockwise")
                        .font(.system(size: 12, weight: .medium))
                        .foregroundColor(SF.Colors.error)
                }
                .buttonStyle(.plain)
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 12)

            let pilots = store.projects.filter { p in
                pilotProjects.contains { $0.name == p.name }
            }
            if !pilots.isEmpty {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 10) {
                        ForEach(pilots) { project in
                            pilotCard(project)
                        }
                    }
                    .padding(.horizontal, 24)
                    .padding(.bottom, 16)
                }
            } else {
                HStack(spacing: 8) {
                    Image(systemName: "info.circle")
                        .foregroundColor(SF.Colors.textMuted)
                    Text("Cliquez \"Charger\" pour importer les 8 projets pilotes")
                        .font(.system(size: 12))
                        .foregroundColor(SF.Colors.textMuted)
                }
                .padding(.horizontal, 24)
                .padding(.bottom, 16)
            }
        }
    }

    private func pilotCard(_ project: Project) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(project.name)
                .font(.system(size: 12, weight: .bold))
                .foregroundColor(SF.Colors.textPrimary)
                .lineLimit(1)
            Text(project.tech)
                .font(.system(size: 10))
                .foregroundColor(SF.Colors.purple)
                .lineLimit(1)
            Text(project.description)
                .font(.system(size: 10))
                .foregroundColor(SF.Colors.textSecondary)
                .lineLimit(2)
            HStack(spacing: 4) {
                Circle()
                    .fill(Color(hex: UInt(project.status.color.dropFirst(), radix: 16) ?? 0x6366f1))
                    .frame(width: 6, height: 6)
                Text(project.status.displayName)
                    .font(.system(size: 9, weight: .medium))
                    .foregroundColor(SF.Colors.textMuted)
            }
        }
        .frame(width: 180)
        .padding(12)
        .background(SF.Colors.bgCard)
        .cornerRadius(SF.Radius.md)
        .overlay(RoundedRectangle(cornerRadius: SF.Radius.md).stroke(SF.Colors.border, lineWidth: 0.5))
    }

    private func loadPilotProjects() {
        for pilot in pilotProjects {
            let exists = store.projects.contains { $0.name == pilot.name }
            if !exists {
                let project = Project(
                    name: pilot.name,
                    description: pilot.description,
                    tech: pilot.tech,
                    status: .idea
                )
                store.add(project)
            }
        }
    }

    private func resetPilotProjects() {
        let pilotNames = Set(pilotProjects.map(\.name))
        let toDelete = store.projects.filter { pilotNames.contains($0.name) }
        for p in toDelete {
            store.delete(p.id)
        }
    }

    private var emptyState: some View {
        VStack(spacing: 16) {
            Image(systemName: "sparkles")
                .font(.system(size: 48))
                .foregroundColor(SF.Colors.purple.opacity(0.4))
            Text("No projects yet")
                .font(.system(size: 18, weight: .semibold))
                .foregroundColor(SF.Colors.textSecondary)
            Text("Ask Jarvis to create a project for you.\n\"Create a project called MyApp using Swift\"")
                .font(.system(size: 13))
                .foregroundColor(SF.Colors.textMuted)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

// MARK: - Project Accordion (card header + collapsible discussion panel)

@MainActor
struct ProjectAccordion: View {
    let project: Project
    let isExpanded: Bool
    let toggle: () -> Void

    @ObservedObject private var bridge = SFBridge.shared
    @ObservedObject private var catalog = SFCatalog.shared
    @State private var missionStatus: SFBridge.MissionStatus?
    @State private var selectedPhaseIndex: Int?
    @State private var pollTimer: Timer?

    /// Events scoped to this project (not the global feed)
    private var projectEvents: [SFBridge.AgentEvent] {
        bridge.eventsForProject(project.id)
    }

    private var activePhase: Int {
        simulatedActivePhase(for: project.status, progress: project.progress)
    }
    private var isActive: Bool { project.status == .active }
    private var isPaused: Bool { project.status == .paused }
    private var isQueued: Bool { project.status == .planning }
    private var isDone: Bool { project.status == .done }
    /// Phases to display: real from mission status, or simulated fallback
    private var displayPhases: [SFBridge.PhaseInfo] {
        if let real = missionStatus?.phases, !real.isEmpty { return real }
        return simulatedPhases()
    }

    /// True when a workflow/mission has been started (or project has progressed beyond idea)
    private var hasWorkflow: Bool {
        project.missionId != nil || project.status != .idea
    }

    var body: some View {
        VStack(spacing: 0) {
            // ── Card header (always visible, clickable to toggle)
            cardHeader
                .contentShape(Rectangle())
                .simultaneousGesture(TapGesture().onEnded { toggle() })

            // ── Expanded: inline discussion panel
            if isExpanded {
                Divider().background(SF.Colors.purple.opacity(0.3))
                discussionPanel
                    .transition(.opacity.combined(with: .move(edge: .top)))
            }
        }
        .background(SF.Colors.bgCard)
        .cornerRadius(SF.Radius.lg)
        .overlay(
            RoundedRectangle(cornerRadius: SF.Radius.lg)
                .stroke(
                    isExpanded ? SF.Colors.purple.opacity(0.6) :
                    isActive ? SF.Colors.purple.opacity(0.4) :
                    SF.Colors.border,
                    lineWidth: isExpanded ? 2 : 1
                )
        )
        .onChange(of: isExpanded) { _, expanded in
            if expanded {
                // Immediate fetch before polling starts
                self.missionStatus = bridge.missionStatusForProject(project.id)
                startPolling()
            } else {
                stopPolling()
            }
        }
    }

    // MARK: - Card Header

    private var cardHeader: some View {
        VStack(alignment: .leading, spacing: 10) {
            // Row 1: chevron + name + tech + status + controls
            HStack(spacing: 8) {
                Image(systemName: isExpanded ? "chevron.down" : "chevron.right")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundColor(SF.Colors.purple)
                    .frame(width: 16)

                Text(project.name)
                    .font(.system(size: 17, weight: .bold))
                    .foregroundColor(SF.Colors.textPrimary)

                if !project.tech.isEmpty {
                    Text(project.tech)
                        .font(.system(size: 11, weight: .medium))
                        .foregroundColor(SF.Colors.textMuted)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(SF.Colors.bgTertiary)
                        .cornerRadius(5)
                }

                Spacer()

                statusIndicator
                controlButtons
            }

            if !project.description.isEmpty && !isExpanded {
                Text(project.description)
                    .font(.system(size: 13))
                    .foregroundColor(SF.Colors.textSecondary)
                    .lineLimit(2)
                    .padding(.leading, 24)
            }

            // Progress bar + Phase timeline — only when a workflow has been started
            if hasWorkflow {
                VStack(alignment: .leading, spacing: 4) {
                    HStack(spacing: 6) {
                        Image(systemName: "flowchart.fill")
                            .font(.system(size: 11))
                            .foregroundColor(SF.Colors.purple.opacity(0.7))
                        Text("Cycle de Vie Produit Complet")
                            .font(.system(size: 11, weight: .medium))
                            .foregroundColor(SF.Colors.textSecondary)
                        Spacer()
                        Text("\(activePhase)/14 phases")
                            .font(.system(size: 11, weight: .semibold))
                            .foregroundColor(SF.Colors.textSecondary)
                    }
                    GeometryReader { geo in
                        ZStack(alignment: .leading) {
                            RoundedRectangle(cornerRadius: 3)
                                .fill(SF.Colors.bgTertiary)
                                .frame(height: 6)
                            RoundedRectangle(cornerRadius: 3)
                                .fill(isDone ? SF.Colors.success : SF.Colors.purple)
                                .frame(width: geo.size.width * CGFloat(activePhase) / 14.0, height: 6)
                        }
                    }
                    .frame(height: 6)
                }
                .padding(.leading, 24)

                // Phase timeline (always visible — clickable when expanded)
                if isExpanded {
                    ClickablePhaseTimeline(
                        activePhase: activePhase,
                        projectDone: isDone,
                        selectedIndex: $selectedPhaseIndex,
                        phases: displayPhases
                    )
                    .padding(.leading, 24)
                } else {
                    MiniPhaseTimeline(activePhase: activePhase, projectDone: isDone)
                        .padding(.leading, 24)
                }
            } else {
                HStack(spacing: 6) {
                    Image(systemName: "questionmark.circle")
                        .font(.system(size: 11))
                        .foregroundColor(SF.Colors.textMuted.opacity(0.5))
                    Text("Workflow non défini — lancez le projet pour que le PM décide du cycle de vie")
                        .font(.system(size: 11))
                        .foregroundColor(SF.Colors.textMuted)
                }
                .padding(.leading, 24)
            }
        }
        .padding(16)
    }

    // MARK: - Discussion Panel (expanded state)

    private var discussionPanel: some View {
        phaseDetailOrFeed
            .frame(minHeight: 200, maxHeight: 400)
    }

    // ── Phase detail or live events ──

    @ViewBuilder
    private var phaseDetailOrFeed: some View {
        let phases = displayPhases
        if let idx = selectedPhaseIndex, idx < phases.count {
            phaseDetailPanel(phases[idx], index: idx)
        } else {
            conversationFeed
        }
    }

    /// Shows the best available conversation: live events > discussion events > persisted > DB > empty
    @ViewBuilder
    private var conversationFeed: some View {
        if !projectEvents.isEmpty {
            liveEventsFeed
        } else if isActive && !bridge.events.isEmpty {
            globalEventsFeed
        } else if isActive && !bridge.discussionEvents.isEmpty {
            // Jarvis discussion events from current session
            eventScrollFeed(events: bridge.discussionEvents)
        } else if let msgs = missionStatus?.messages, !msgs.isEmpty {
            missionMessagesFeed(msgs)
        } else {
            // Try DB: first match project name, then fall back to most recent session with messages
            let dbMsgs = bridge.discussionMessagesForProject(project.name)
            if !dbMsgs.isEmpty {
                discussionMessagesFeed(dbMsgs)
            } else if isActive {
                let allMsgs = bridge.mostRecentDiscussionMessages()
                if !allMsgs.isEmpty {
                    discussionMessagesFeed(allMsgs)
                } else {
                    activeNoDataPlaceholder
                }
            } else {
                emptyDiscussionPlaceholder
            }
        }
    }

    private func phaseDetailPanel(_ phase: SFBridge.PhaseInfo, index: Int) -> some View {
        VStack(spacing: 0) {
            // Phase header
            HStack(spacing: 10) {
                ZStack {
                    Circle()
                        .fill(phaseCircleFill(status: phase.status))
                        .frame(width: 28, height: 28)
                    if phase.status == "completed" {
                        Image(systemName: "checkmark").font(.system(size: 10, weight: .bold)).foregroundColor(.white)
                    } else {
                        Text("\(index + 1)").font(.system(size: 10, weight: .bold)).foregroundColor(.white)
                    }
                }

                VStack(alignment: .leading, spacing: 2) {
                    Text(phase.phase_name)
                        .font(.system(size: 14, weight: .bold))
                        .foregroundColor(SF.Colors.textPrimary)
                    HStack(spacing: 6) {
                        PatternBadge(pattern: phase.pattern)
                        phaseStatusChip(phase.status)
                    }
                }

                Spacer()

                agentAvatarStack(phase.agent_ids)

                Button(action: { selectedPhaseIndex = nil }) {
                    Image(systemName: "xmark.circle.fill")
                        .font(.system(size: 14))
                        .foregroundColor(SF.Colors.textMuted)
                }
                .buttonStyle(.plain)
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 10)
            .background(SF.Colors.bgSecondary)

            Divider().background(SF.Colors.border)

            // Phase messages
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 8) {
                    let phaseMessages = messagesForPhase(phase)
                    if phaseMessages.isEmpty {
                        HStack {
                            Spacer()
                            VStack(spacing: 6) {
                                Image(systemName: phase.status == "pending" ? "clock" : "bubble.left.and.bubble.right")
                                    .font(.system(size: 24))
                                    .foregroundColor(SF.Colors.textMuted)
                                Text(phase.status == "pending" ? "Phase en attente" : "Discussion en cours…")
                                    .font(.system(size: 12))
                                    .foregroundColor(SF.Colors.textMuted)
                            }
                            .padding(.top, 30)
                            Spacer()
                        }
                    } else {
                        ForEach(phaseMessages) { msg in
                            phaseMessageCard(msg, pattern: phase.pattern, phaseAgentIds: phase.agent_ids)
                        }
                    }

                    // Phase output
                    if let output = phase.output, !output.isEmpty {
                        VStack(alignment: .leading, spacing: 6) {
                            HStack(spacing: 5) {
                                Image(systemName: "doc.text")
                                    .font(.system(size: 11))
                                    .foregroundColor(SF.Colors.textSecondary)
                                Text("Résultat de phase")
                                    .font(.system(size: 11, weight: .semibold))
                                    .foregroundColor(SF.Colors.textSecondary)
                            }
                            MarkdownView(output, fontSize: 12)
                                .textSelection(.enabled)
                        }
                        .padding(12)
                        .background(SF.Colors.bgTertiary)
                        .cornerRadius(8)
                    }
                }
                .padding(16)
            }
        }
    }

    private func phaseMessageCard(_ msg: SFBridge.MessageInfo, pattern: String, phaseAgentIds: String) -> some View {
        let aid = msg.role
        let color = catalog.agentColor(aid)
        let agentRole = catalog.agentRole(aid)
        let recipients: [String] = {
            guard let data = phaseAgentIds.data(using: .utf8),
                  let arr = try? JSONDecoder().decode([String].self, from: data) else { return [] }
            return arr.filter { $0 != aid }
        }()

        return HStack(alignment: .top, spacing: 0) {
            RoundedRectangle(cornerRadius: 2)
                .fill(color)
                .frame(width: 3)

            HStack(alignment: .top, spacing: 10) {
                AgentAvatarView(agentId: aid, size: 36)
                    .overlay(Circle().stroke(color.opacity(0.5), lineWidth: 2))

                VStack(alignment: .leading, spacing: 5) {
                    HStack(spacing: 5) {
                        Text(msg.agent_name)
                            .font(.system(size: 13, weight: .bold))
                            .foregroundColor(color)
                        if !agentRole.isEmpty {
                            Text(agentRole)
                                .font(.system(size: 9, weight: .medium))
                                .foregroundColor(SF.Colors.textSecondary)
                                .padding(.horizontal, 5)
                                .padding(.vertical, 1)
                                .background(color.opacity(0.1))
                                .cornerRadius(3)
                        }
                        PatternBadge(pattern: pattern)
                        Spacer()
                        Text(msg.created_at.suffix(8))
                            .font(.system(size: 9))
                            .foregroundColor(SF.Colors.textMuted)
                    }

                    if !recipients.isEmpty {
                        HStack(spacing: 3) {
                            Image(systemName: "arrow.right")
                                .font(.system(size: 8))
                                .foregroundColor(SF.Colors.textMuted)
                            ForEach(recipients.prefix(3), id: \.self) { rid in
                                HStack(spacing: 2) {
                                    AgentAvatarView(agentId: rid, size: 14)
                                    Text(catalog.agentName(rid))
                                        .font(.system(size: 9))
                                        .foregroundColor(SF.Colors.textSecondary)
                                }
                            }
                            if recipients.count > 3 {
                                Text("+\(recipients.count - 3)")
                                    .font(.system(size: 9))
                                    .foregroundColor(SF.Colors.textMuted)
                            }
                        }
                    }

                    MarkdownView(msg.content, fontSize: 12)
                        .textSelection(.enabled)
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 10)
        }
        .background(SF.Colors.bgCard)
        .cornerRadius(8)
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(SF.Colors.border.opacity(0.3), lineWidth: 0.5)
        )
    }

    // ── Mission messages feed (persisted conversation from completed/running phases) ──

    private func missionMessagesFeed(_ messages: [SFBridge.MessageInfo]) -> some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 8) {
                ForEach(Array(messages.enumerated()), id: \.offset) { _, msg in
                    missionMessageRow(msg)
                }
            }
            .padding(16)
        }
    }

    private func missionMessageRow(_ msg: SFBridge.MessageInfo) -> some View {
        let color = catalog.agentColor(msg.agent_name)
        return HStack(alignment: .top, spacing: 8) {
            AgentAvatarView(agentId: msg.agent_name, size: 28)
                .overlay(Circle().stroke(color.opacity(0.4), lineWidth: 1))
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 5) {
                    Text(catalog.agentName(msg.agent_name))
                        .font(.system(size: 12, weight: .bold))
                        .foregroundColor(color)
                    RoleBadge(role: msg.role, color: color)
                    Spacer()
                    Text(String(msg.created_at.suffix(8)))
                        .font(.system(size: 9, weight: .medium, design: .monospaced))
                        .foregroundColor(SF.Colors.textMuted)
                }
                Text(String(msg.content.prefix(500)))
                    .font(.system(size: 12))
                    .foregroundColor(SF.Colors.textPrimary)
                    .lineLimit(6)
                    .textSelection(.enabled)
            }
        }
        .padding(10)
        .background(SF.Colors.bgSecondary.opacity(0.5))
        .cornerRadius(8)
    }

    // ── Discussion messages feed (from Rust engine DB) ──

    private func discussionMessagesFeed(_ messages: [SFBridge.DiscussionMessage]) -> some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 8) {
                ForEach(messages) { msg in
                    discussionMessageRow(msg)
                }
            }
            .padding(16)
        }
    }

    private func discussionMessageRow(_ msg: SFBridge.DiscussionMessage) -> some View {
        let color = catalog.agentColor(msg.agentId)
        return HStack(alignment: .top, spacing: 8) {
            AgentAvatarView(agentId: msg.agentId, size: 28)
                .overlay(Circle().stroke(color.opacity(0.4), lineWidth: 1))
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 5) {
                    Text(msg.agentName)
                        .font(.system(size: 12, weight: .bold))
                        .foregroundColor(color)
                    RoleBadge(role: msg.agentRole, color: color)
                    Text("Tour \(msg.round)")
                        .font(.system(size: 9, weight: .medium))
                        .foregroundColor(SF.Colors.textMuted)
                        .padding(.horizontal, 5)
                        .padding(.vertical, 1)
                        .background(SF.Colors.bgTertiary)
                        .cornerRadius(4)
                    Spacer()
                    Text(String(msg.createdAt.suffix(8)))
                        .font(.system(size: 9, weight: .medium, design: .monospaced))
                        .foregroundColor(SF.Colors.textMuted)
                }
                MarkdownView(msg.content, fontSize: 12)
                    .textSelection(.enabled)
            }
        }
        .padding(10)
        .background(SF.Colors.bgSecondary.opacity(0.5))
        .cornerRadius(8)
    }

    // ── Live events feed ──

    private var emptyDiscussionPlaceholder: some View {
        HStack {
            Spacer()
            VStack(spacing: 8) {
                Image(systemName: "play.circle")
                    .font(.system(size: 32))
                    .foregroundColor(SF.Colors.textMuted.opacity(0.5))
                Text("Lancez le workflow pour voir la discussion des agents")
                    .font(.system(size: 13))
                    .foregroundColor(SF.Colors.textMuted)
            }
            .padding(.top, 30)
            Spacer()
        }
    }

    private var activeNoDataPlaceholder: some View {
        HStack {
            Spacer()
            VStack(spacing: 8) {
                ProgressView()
                    .scaleEffect(0.8)
                    .tint(SF.Colors.purple)
                Text("Les agents travaillent — la conversation apparaîtra ici")
                    .font(.system(size: 13))
                    .foregroundColor(SF.Colors.textMuted)
            }
            .padding(.top, 30)
            Spacer()
        }
    }

    private var liveEventsFeed: some View {
        eventScrollFeed(events: projectEvents)
    }

    private var globalEventsFeed: some View {
        eventScrollFeed(events: bridge.events)
    }

    /// Pre-process events: collapse tool_call/tool_result into compact groups,
    /// filter out thinking events (shown inline), count only substantive messages.
    private struct DisplayItem: Identifiable {
        let id: String
        enum Kind {
            case message(SFBridge.AgentEvent)
            case toolGroup([SFBridge.AgentEvent])
        }
        let kind: Kind
    }

    private func buildDisplayItems(_ raw: [SFBridge.AgentEvent]) -> [DisplayItem] {
        var items: [DisplayItem] = []
        var pendingTools: [SFBridge.AgentEvent] = []
        // Track agents that have produced a response — their thinking events are stale
        let agentsWithResponse = Set(raw.filter {
            $0.eventType == "response" || $0.eventType == "discuss_response" || $0.eventType == "response_chunk"
        }.map { $0.agentId })

        func flushTools() {
            guard !pendingTools.isEmpty else { return }
            let id = pendingTools.first!.id.uuidString
            items.append(DisplayItem(id: "tg-\(id)", kind: .toolGroup(pendingTools)))
            pendingTools = []
        }

        for event in raw {
            // Skip thinking events if this agent already has a response
            if (event.eventType == "thinking" || event.eventType == "discuss_thinking"),
               agentsWithResponse.contains(event.agentId) {
                continue
            }
            if event.eventType == "tool_call" || event.eventType == "tool_result" {
                pendingTools.append(event)
            } else {
                flushTools()
                items.append(DisplayItem(id: event.id.uuidString, kind: .message(event)))
            }
        }
        flushTools()
        return items
    }

    private func eventScrollFeed(events feedEvents: [SFBridge.AgentEvent]) -> some View {
        let displayItems = buildDisplayItems(feedEvents)
        let messageCount = feedEvents.filter { $0.eventType == "response" || $0.eventType == "discuss_response" }.count

        return ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 6) {
                    // Pattern header
                    if let pat = currentPhasePattern, !pat.isEmpty {
                        HStack(spacing: 6) {
                            PatternBadge(pattern: pat)
                            Text("·")
                                .foregroundColor(SF.Colors.textMuted)
                            Text("\(messageCount) messages")
                                .font(.system(size: 10))
                                .foregroundColor(SF.Colors.textMuted)
                            Spacer()
                        }
                        .padding(.bottom, 4)
                    }

                    ForEach(displayItems) { item in
                        switch item.kind {
                        case .message(let event):
                            eventRow(event, isLast: item.id == displayItems.last?.id && !(isActive && bridge.isRunning))
                                .id(item.id)
                        case .toolGroup(let tools):
                            toolBadgeRow(tools)
                                .id(item.id)
                        }
                    }

                    // Streaming indicator — only when truly active
                    if isActive && bridge.isRunning {
                        streamingIndicator.id("streaming-tail")
                    }
                }
                .padding(16)
            }
            .onChange(of: feedEvents.count) { _, _ in
                withAnimation {
                    if isActive && bridge.isRunning {
                        proxy.scrollTo("streaming-tail", anchor: .bottom)
                    } else if let last = displayItems.last {
                        proxy.scrollTo(last.id, anchor: .bottom)
                    }
                }
            }
        }
    }

    private var streamingIndicator: some View {
        HStack(spacing: 8) {
            ProgressView()
                .controlSize(.small)
                .tint(SF.Colors.purple)
            Text(bridge.isReasoning ? "Raisonnement en profondeur…" : "Agents en cours de réflexion…")
                .font(.system(size: 11, weight: .medium))
                .foregroundColor(bridge.isReasoning ? SF.Colors.purple : SF.Colors.textMuted)
            Spacer()
        }
        .padding(10)
        .background(SF.Colors.purple.opacity(bridge.isReasoning ? 0.12 : 0.06))
        .cornerRadius(6)
    }

    /// Compact inline badges for tool_call / tool_result groups
    private func toolBadgeRow(_ tools: [SFBridge.AgentEvent]) -> some View {
        let badges: [(icon: String, label: String)] = tools.compactMap { event in
            let raw = event.data
            // tool_call format: "tool_name|args"  tool_result format: "tool_name|result"
            let toolName = raw.components(separatedBy: "|").first?.trimmingCharacters(in: .whitespaces) ?? raw
            let shortName = toolName
                .replacingOccurrences(of: "code_", with: "")
                .replacingOccurrences(of: "file_", with: "")
                .replacingOccurrences(of: "memory_", with: "mem:")
                .replacingOccurrences(of: "git_", with: "git:")
                .replacingOccurrences(of: "deep_", with: "")
                .replacingOccurrences(of: "list_", with: "ls:")

            let icon: String
            if event.eventType == "tool_call" {
                switch true {
                case toolName.contains("read"):   icon = "doc.text"
                case toolName.contains("write"):  icon = "square.and.pencil"
                case toolName.contains("edit"):   icon = "pencil"
                case toolName.contains("search"): icon = "magnifyingglass"
                case toolName.contains("list"):   icon = "list.bullet"
                case toolName.contains("git"):    icon = "arrow.triangle.branch"
                case toolName.contains("memory"): icon = "brain"
                case toolName.contains("build"):  icon = "hammer"
                case toolName.contains("test"):   icon = "checkmark.shield"
                default:                           icon = "wrench"
                }
                return (icon, shortName)
            }
            return nil // skip tool_result (already shown via tool_call badge)
        }

        guard !badges.isEmpty else { return AnyView(EmptyView()) }

        return AnyView(
            HStack(spacing: 4) {
                Image(systemName: "gearshape.2")
                    .font(.system(size: 9))
                    .foregroundColor(SF.Colors.textMuted)
                ForEach(Array(badges.enumerated()), id: \.offset) { _, badge in
                    HStack(spacing: 3) {
                        Image(systemName: badge.icon)
                            .font(.system(size: 9))
                        Text(badge.label)
                            .font(.system(size: 9, weight: .medium, design: .monospaced))
                    }
                    .foregroundColor(SF.Colors.textSecondary)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 3)
                    .background(SF.Colors.bgTertiary)
                    .cornerRadius(4)
                }
                Spacer()
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 2)
        )
    }

    private func eventRow(_ event: SFBridge.AgentEvent, isLast: Bool = false) -> some View {
        let color = catalog.agentColor(event.agentId)
        let displayRole = !event.role.isEmpty ? event.role : catalog.agentRole(event.agentId)
        let displayName = !event.agentName.isEmpty ? event.agentName : catalog.agentName(event.agentId)
        let mtype = event.messageType.isEmpty ? "response" : event.messageType
        let borderColor = eventBorderColor(mtype)

        // Thinking events: compact indicator (same as Jarvis)
        if event.eventType == "thinking" || event.eventType == "discuss_thinking" {
            return AnyView(thinkingRow(event))
        }

        // Full card (Jarvis-style, scaled down)
        return AnyView(
            HStack(alignment: .top, spacing: 0) {
                // Left accent border
                RoundedRectangle(cornerRadius: 2)
                    .fill(borderColor)
                    .frame(width: 3)

                VStack(alignment: .leading, spacing: 8) {
                    // ── Metadata header ──
                    HStack(spacing: 8) {
                        AgentAvatarView(agentId: event.agentId, size: 32)
                            .overlay(Circle().stroke(color.opacity(0.6), lineWidth: 2))

                        VStack(alignment: .leading, spacing: 2) {
                            HStack(spacing: 6) {
                                Text(displayName)
                                    .font(.system(size: 13, weight: .bold))
                                    .foregroundColor(color)
                                if mtype != "response" {
                                    eventTypeBadge(mtype)
                                }
                                if event.round > 0 {
                                    Text("R\(event.round)")
                                        .font(.system(size: 9, weight: .bold, design: .monospaced))
                                        .foregroundColor(SF.Colors.purple)
                                        .padding(.horizontal, 4)
                                        .padding(.vertical, 2)
                                        .background(SF.Colors.purple.opacity(0.1))
                                        .cornerRadius(4)
                                }
                            }
                            HStack(spacing: 5) {
                                if !displayRole.isEmpty {
                                    Text(displayRole)
                                        .font(.system(size: 11).italic())
                                        .foregroundColor(SF.Colors.textSecondary)
                                }
                                // Current phase pattern
                                if let pat = currentPhasePattern, !pat.isEmpty {
                                    PatternBadge(pattern: pat)
                                }
                            }
                        }

                        Spacer()

                        // Recipients (right side)
                        if !event.toAgents.isEmpty {
                            eventRecipientsView(event.toAgents)
                        }

                        Text(event.timestamp, style: .time)
                            .font(.system(size: 9))
                            .foregroundColor(SF.Colors.textMuted)
                    }

                    Divider().background(SF.Colors.border.opacity(0.5))

                    // ── Content ──
                    if !event.data.isEmpty {
                        let cleanContent = Self.stripToolCallLines(event.data)
                        if !cleanContent.isEmpty {
                            MarkdownView(cleanContent, fontSize: 12)
                                .textSelection(.enabled)
                        }
                        // Inline tool badges extracted from [Calling tools: ...] lines
                        let inlineTools = Self.extractInlineTools(event.data)
                        if !inlineTools.isEmpty {
                            HStack(spacing: 4) {
                                Image(systemName: "wrench.and.screwdriver")
                                    .font(.system(size: 9))
                                    .foregroundColor(SF.Colors.textMuted)
                                ForEach(Array(inlineTools.enumerated()), id: \.offset) { _, tool in
                                    HStack(spacing: 3) {
                                        Image(systemName: Self.toolIcon(tool))
                                            .font(.system(size: 9))
                                        Text(tool)
                                            .font(.system(size: 9, weight: .medium, design: .monospaced))
                                    }
                                    .foregroundColor(SF.Colors.textSecondary)
                                    .padding(.horizontal, 6)
                                    .padding(.vertical, 3)
                                    .background(SF.Colors.bgTertiary)
                                    .cornerRadius(4)
                                }
                                Spacer()
                            }
                        }
                    }

                    // Typing cursor for last active event
                    if isLast && isActive && bridge.isRunning {
                        HStack(spacing: 3) {
                            Circle().fill(SF.Colors.purple).frame(width: 4, height: 4)
                                .modifier(PulseAnimation())
                            Circle().fill(SF.Colors.purple).frame(width: 4, height: 4)
                                .modifier(PulseAnimation(delay: 0.2))
                            Circle().fill(SF.Colors.purple).frame(width: 4, height: 4)
                                .modifier(PulseAnimation(delay: 0.4))
                        }
                    }
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 12)
            }
            .background(SF.Colors.bgCard)
            .cornerRadius(10)
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .stroke(SF.Colors.border.opacity(0.5), lineWidth: 0.5)
            )
        )
    }

    /// Current phase pattern (for display in event cards)
    private var currentPhasePattern: String? {
        let phases = displayPhases
        if let running = phases.first(where: { $0.status == "running" }) { return running.pattern }
        return nil
    }

    private func thinkingRow(_ event: SFBridge.AgentEvent) -> some View {
        let name = !event.agentName.isEmpty ? event.agentName : catalog.agentName(event.agentId)
        return HStack(spacing: 10) {
            AgentAvatarView(agentId: event.agentId, size: 28)
            ProgressView().controlSize(.small)
            Text("\(name) rédige…")
                .font(.system(size: 12))
                .foregroundColor(SF.Colors.textMuted)
            Spacer()
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 6)
    }

    @ViewBuilder
    private func eventTypeBadge(_ type: String) -> some View {
        let (bg, fg) = eventBadgeColors(type)
        Text(type.uppercased())
            .font(.system(size: 9, weight: .bold))
            .foregroundColor(fg)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(bg)
            .cornerRadius(4)
    }

    // MARK: - Tool call text → badge conversion

    /// Strip "[Calling tools: ...]" lines and standalone tool call text from content
    private static func stripToolCallLines(_ text: String) -> String {
        text.components(separatedBy: "\n")
            .filter { line in
                let trimmed = line.trimmingCharacters(in: .whitespaces)
                if trimmed.hasPrefix("[Calling tools:") && trimmed.hasSuffix("]") { return false }
                if trimmed.hasPrefix("[Tool ") && trimmed.contains("result:") { return false }
                return true
            }
            .joined(separator: "\n")
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// Extract tool names from "[Calling tools: name(args), name2(args)]" lines
    private static func extractInlineTools(_ text: String) -> [String] {
        var tools: [String] = []
        for line in text.components(separatedBy: "\n") {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            guard trimmed.hasPrefix("[Calling tools:"), trimmed.hasSuffix("]") else { continue }
            let inner = String(trimmed.dropFirst("[Calling tools:".count).dropLast())
                .trimmingCharacters(in: .whitespaces)
            // Parse "name(args), name2(args)"
            for part in inner.components(separatedBy: ",") {
                let name = part.trimmingCharacters(in: .whitespaces)
                    .components(separatedBy: "(").first?
                    .trimmingCharacters(in: .whitespaces) ?? ""
                if !name.isEmpty {
                    let short = name
                        .replacingOccurrences(of: "code_", with: "")
                        .replacingOccurrences(of: "file_", with: "")
                        .replacingOccurrences(of: "memory_", with: "mem:")
                        .replacingOccurrences(of: "git_", with: "git:")
                        .replacingOccurrences(of: "list_", with: "ls:")
                        .replacingOccurrences(of: "deep_", with: "")
                    tools.append(short)
                }
            }
        }
        return tools
    }

    private static func toolIcon(_ name: String) -> String {
        switch true {
        case name.contains("read"):   return "doc.text"
        case name.contains("write"):  return "square.and.pencil"
        case name.contains("edit"):   return "pencil"
        case name.contains("search"): return "magnifyingglass"
        case name.hasPrefix("ls:"):   return "list.bullet"
        case name.hasPrefix("git:"):  return "arrow.triangle.branch"
        case name.hasPrefix("mem:"):  return "brain"
        default:                       return "wrench"
        }
    }

    @ViewBuilder
    private func eventRecipientsView(_ toAgents: [String]) -> some View {
        HStack(spacing: 3) {
            Image(systemName: "arrow.right")
                .font(.system(size: 9, weight: .medium))
                .foregroundColor(SF.Colors.textMuted)
            ForEach(toAgents, id: \.self) { agentId in
                let displayName = agentId == "all" ? "Tous" : catalog.agentName(agentId)
                let c = catalog.agentColor(agentId)
                HStack(spacing: 3) {
                    AgentAvatarView(agentId: agentId, size: 16)
                    Text(displayName)
                        .font(.system(size: 10, weight: .medium))
                        .foregroundColor(c)
                }
                .padding(.horizontal, 4)
                .padding(.vertical, 2)
                .background(c.opacity(0.1))
                .cornerRadius(4)
            }
        }
    }

    private func eventBorderColor(_ type: String) -> Color {
        switch type {
        case "instruction", "delegation": return SF.Colors.yellowDeep
        case "response", "approval":      return Color(red: 0.13, green: 0.77, blue: 0.37)
        case "veto":                      return SF.Colors.error
        case "synthesis":                 return SF.Colors.po
        default:                          return SF.Colors.textMuted.opacity(0.5)
        }
    }

    private func eventBadgeColors(_ type: String) -> (Color, Color) {
        switch type {
        case "instruction":  return (SF.Colors.yellowDeep.opacity(0.2), SF.Colors.yellowDeep)
        case "delegation":   return (SF.Colors.purple.opacity(0.2), SF.Colors.purple)
        case "approval":     return (Color(red: 0.13, green: 0.77, blue: 0.37).opacity(0.2), Color(red: 0.13, green: 0.77, blue: 0.37))
        case "veto":         return (SF.Colors.error.opacity(0.2), SF.Colors.error)
        case "synthesis":    return (SF.Colors.po.opacity(0.2), SF.Colors.po)
        default:             return (SF.Colors.textMuted.opacity(0.15), SF.Colors.textMuted)
        }
    }

    // MARK: - Helpers

    private func simulatedPhases() -> [SFBridge.PhaseInfo] {
        safePhases.enumerated().map { i, p in
            SFBridge.PhaseInfo(
                id: "sim-\(i)",
                phase_name: p.name,
                pattern: p.pattern,
                status: i < activePhase ? "completed" : (i == activePhase && isActive ? "running" : "pending"),
                agent_ids: "[]",
                output: nil,
                started_at: nil,
                completed_at: nil
            )
        }
    }

    private func messagesForPhase(_ phase: SFBridge.PhaseInfo) -> [SFBridge.MessageInfo] {
        guard let messages = missionStatus?.messages else { return [] }
        let ids: [String] = {
            guard let data = phase.agent_ids.data(using: .utf8),
                  let arr = try? JSONDecoder().decode([String].self, from: data) else { return [] }
            return arr
        }()
        if ids.isEmpty { return Array(messages.reversed()) }
        return messages.reversed().filter { msg in
            ids.contains(msg.role) || ids.contains(msg.agent_name.lowercased())
        }
    }

    private func agentAvatarStack(_ agentIdsJson: String) -> some View {
        let ids: [String] = {
            guard let data = agentIdsJson.data(using: .utf8),
                  let arr = try? JSONDecoder().decode([String].self, from: data) else { return [] }
            return arr
        }()
        return HStack(spacing: -6) {
            ForEach(ids.prefix(4), id: \.self) { aid in
                AgentAvatarView(agentId: aid, size: 24)
                    .overlay(Circle().stroke(SF.Colors.bgSecondary, lineWidth: 1.5))
            }
            if ids.count > 4 {
                Text("+\(ids.count - 4)")
                    .font(.system(size: 9, weight: .bold))
                    .foregroundColor(SF.Colors.textSecondary)
                    .frame(width: 24, height: 24)
                    .background(SF.Colors.bgTertiary)
                    .clipShape(Circle())
            }
        }
    }

    private func phaseCircleFill(status: String) -> Color {
        switch status {
        case "completed": return SF.Colors.success
        case "running":   return SF.Colors.purple
        case "failed":    return SF.Colors.error
        case "vetoed":    return SF.Colors.warning
        default:          return SF.Colors.bgTertiary
        }
    }

    private func phaseStatusChip(_ status: String) -> some View {
        let (label, color): (String, Color) = {
            switch status {
            case "completed": return ("✓ Terminé", SF.Colors.success)
            case "running":   return ("⏳ En cours", SF.Colors.purple)
            case "failed":    return ("✗ Échoué", SF.Colors.error)
            case "vetoed":    return ("⚠ Véto", SF.Colors.warning)
            default:          return ("En attente", SF.Colors.textMuted)
            }
        }()
        return Text(label)
            .font(.system(size: 9, weight: .semibold))
            .foregroundColor(color)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(color.opacity(0.1))
            .cornerRadius(4)
    }

    private func patternColor(_ pattern: String) -> Color {
        switch pattern {
        case "network":           return SF.Colors.info
        case "sequential":        return SF.Colors.success
        case "parallel":          return .cyan
        case "hierarchical":      return SF.Colors.purple
        case "loop":              return SF.Colors.warning
        case "aggregator":        return .teal
        case "human-in-the-loop": return SF.Colors.accent
        case "router":            return .mint
        default:                  return SF.Colors.textMuted
        }
    }

    private func phaseShortName(_ name: String) -> String {
        let map: [String: String] = [
            "Idéation": "Idéation", "Comité Stratégique": "Comité Strat.",
            "Constitution": "Constitution", "Architecture": "Architecture",
            "Design System": "Design Sys.", "Sprints Dev": "Sprints Dev",
            "Build & Verify": "Build & Verify", "Pipeline CI/CD": "Pipeline CI",
            "Revue UX": "Revue UX", "Campagne QA": "Campagne QA",
            "Exécution Tests": "Exécution", "Deploy Prod": "Deploy Prod",
            "Routage TMA": "Routage", "Correctif & TMA": "Correctif",
        ]
        return map[name] ?? name
    }

    private func messageTypeColor(_ type: String) -> Color {
        switch type {
        case "instruction", "delegation": return SF.Colors.yellowDeep
        case "approval":                  return SF.Colors.success
        case "veto":                      return SF.Colors.error
        case "synthesis":                 return SF.Colors.po
        default:                          return SF.Colors.textMuted
        }
    }

    // ── Status indicator ──
    @ViewBuilder
    private var statusIndicator: some View {
        if isActive && bridge.isRunning {
            HStack(spacing: 5) {
                ProgressView().scaleEffect(0.55)
                Text("Agents en cours…")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundColor(SF.Colors.purple)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(SF.Colors.purple.opacity(0.1))
            .cornerRadius(6)
        } else if isActive {
            HStack(spacing: 5) {
                Image(systemName: "circle.fill")
                    .font(.system(size: 6))
                    .foregroundColor(SF.Colors.success)
                Text("Prêt")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundColor(SF.Colors.success)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(SF.Colors.success.opacity(0.1))
            .cornerRadius(6)
        } else if isQueued {
            HStack(spacing: 5) {
                Image(systemName: "clock.fill").font(.system(size: 10))
                Text("Queued").font(.system(size: 11, weight: .semibold))
            }
            .foregroundColor(SF.Colors.warning)
            .padding(.horizontal, 10).padding(.vertical, 5)
            .background(SF.Colors.warning.opacity(0.1))
            .cornerRadius(6)
        } else if isPaused {
            HStack(spacing: 5) {
                Image(systemName: "pause.circle.fill").font(.system(size: 10))
                Text("Paused").font(.system(size: 11, weight: .semibold))
            }
            .foregroundColor(SF.Colors.warning)
            .padding(.horizontal, 10).padding(.vertical, 5)
            .background(SF.Colors.warning.opacity(0.1))
            .cornerRadius(6)
        } else if isDone {
            HStack(spacing: 5) {
                Image(systemName: "checkmark.circle.fill").font(.system(size: 10))
                Text("Terminé").font(.system(size: 11, weight: .semibold))
            }
            .foregroundColor(SF.Colors.success)
            .padding(.horizontal, 10).padding(.vertical, 5)
            .background(SF.Colors.success.opacity(0.1))
            .cornerRadius(6)
        } else {
            Text(project.status.displayName)
                .font(.system(size: 11, weight: .semibold))
                .foregroundColor(Color(hex: project.status.color))
                .padding(.horizontal, 10).padding(.vertical, 5)
                .background(Color(hex: project.status.color).opacity(0.12))
                .cornerRadius(6)
        }
    }

    // ── Play / Pause / Stop ──
    private var controlButtons: some View {
        HStack(spacing: 4) {
            if isActive {
                Button(action: { ProjectStore.shared.setStatus(project.id, status: .paused) }) {
                    Image(systemName: "pause.fill").font(.system(size: 12)).foregroundColor(SF.Colors.warning)
                        .frame(width: 28, height: 28).background(SF.Colors.warning.opacity(0.12)).cornerRadius(6)
                }.buttonStyle(.plain)
                Button(action: { ProjectStore.shared.setStatus(project.id, status: .idea) }) {
                    Image(systemName: "stop.fill").font(.system(size: 12)).foregroundColor(SF.Colors.error)
                        .frame(width: 28, height: 28).background(SF.Colors.error.opacity(0.12)).cornerRadius(6)
                }.buttonStyle(.plain)
            } else if isPaused {
                Button(action: { resumeProject() }) {
                    Image(systemName: "play.fill").font(.system(size: 12)).foregroundColor(SF.Colors.success)
                        .frame(width: 28, height: 28).background(SF.Colors.success.opacity(0.12)).cornerRadius(6)
                }.buttonStyle(.plain)
                Button(action: { ProjectStore.shared.setStatus(project.id, status: .idea) }) {
                    Image(systemName: "stop.fill").font(.system(size: 12)).foregroundColor(SF.Colors.error)
                        .frame(width: 28, height: 28).background(SF.Colors.error.opacity(0.12)).cornerRadius(6)
                }.buttonStyle(.plain)
            } else if !isDone {
                Button(action: { launchProject() }) {
                    Image(systemName: "play.fill").font(.system(size: 12)).foregroundColor(SF.Colors.success)
                        .frame(width: 28, height: 28).background(SF.Colors.success.opacity(0.12)).cornerRadius(6)
                }.buttonStyle(.plain)
            }
        }
    }

    private func launchProject() {
        ProjectStore.shared.setStatus(project.id, status: .active)
        Task {
            await bridge.syncLLMConfigAsync()
            bridge.startMissionAsync(projectId: project.id, brief: project.description)
        }
    }

    private func resumeProject() {
        ProjectStore.shared.setStatus(project.id, status: .active)
        Task {
            await bridge.syncLLMConfigAsync()
            bridge.startMissionAsync(projectId: project.id, brief: project.description)
        }
    }

    private func startPolling() {
        pollTimer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { _ in
            Task { @MainActor in
                self.missionStatus = bridge.missionStatusForProject(project.id)
            }
        }
    }

    private func stopPolling() {
        pollTimer?.invalidate()
        pollTimer = nil
    }
}

// MARK: - Clickable Phase Timeline (expanded card — dots select phase)

struct ClickablePhaseTimeline: View {
    let activePhase: Int
    let projectDone: Bool
    @Binding var selectedIndex: Int?
    let phases: [SFBridge.PhaseInfo]

    private let dotSize: CGFloat = 26
    private let labelWidth: CGFloat = 58
    private let connectorWidth: CGFloat = 10

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 0) {
                ForEach(0..<phases.count, id: \.self) { i in
                    HStack(spacing: 0) {
                        phaseDot(index: i, phase: phases[i])
                        if i < phases.count - 1 {
                            phaseConnector(index: i)
                        }
                    }
                }
            }
            .padding(.horizontal, 4)
            .padding(.vertical, 4)
        }
        .frame(height: 58)
    }

    private func phaseDot(index: Int, phase: SFBridge.PhaseInfo) -> some View {
        let isCompleted = phase.status == "completed"
        let isRunning = phase.status == "running"
        let isFailed = phase.status == "failed" || phase.status == "vetoed"
        let isSelected = selectedIndex == index

        return Button(action: {
            withAnimation(.easeInOut(duration: 0.15)) {
                selectedIndex = selectedIndex == index ? nil : index
            }
        }) {
            VStack(spacing: 3) {
                ZStack {
                    Circle()
                        .fill(dotFill(completed: isCompleted || projectDone, active: isRunning, failed: isFailed))
                        .frame(width: dotSize, height: dotSize)

                    if isSelected {
                        Circle()
                            .stroke(SF.Colors.purple, lineWidth: 2.5)
                            .frame(width: dotSize + 6, height: dotSize + 6)
                    } else if isRunning {
                        Circle()
                            .stroke(SF.Colors.purple.opacity(0.6), lineWidth: 2)
                            .frame(width: dotSize + 5, height: dotSize + 5)
                    }

                    if isCompleted || projectDone {
                        Image(systemName: "checkmark")
                            .font(.system(size: 10, weight: .bold))
                            .foregroundColor(.white)
                    } else if isFailed {
                        Image(systemName: "xmark")
                            .font(.system(size: 10, weight: .bold))
                            .foregroundColor(.white)
                    } else if isRunning {
                        ProgressView().scaleEffect(0.45).tint(.white)
                    } else {
                        Text("\(index + 1)")
                            .font(.system(size: 10, weight: .bold))
                            .foregroundColor(isSelected ? .white : SF.Colors.textMuted)
                    }
                }

                Text(safePhases[safe: index]?.short ?? phase.phase_name)
                    .font(.system(size: 8, weight: isSelected ? .bold : .medium))
                    .foregroundColor(
                        isSelected ? SF.Colors.purple :
                        (isCompleted || projectDone) ? SF.Colors.textSecondary :
                        isRunning ? SF.Colors.purple :
                        SF.Colors.textMuted.opacity(0.5)
                    )
                    .lineLimit(1)
                    .frame(width: labelWidth)
            }
        }
        .buttonStyle(.plain)
    }

    private func phaseConnector(index: Int) -> some View {
        let isCompleted = index < activePhase || projectDone
        return Rectangle()
            .fill(isCompleted ? SF.Colors.success.opacity(0.5) : SF.Colors.border.opacity(0.4))
            .frame(width: connectorWidth, height: 2)
            .padding(.bottom, 16)
    }

    private func dotFill(completed: Bool, active: Bool, failed: Bool) -> Color {
        if completed { return SF.Colors.success }
        if failed    { return SF.Colors.error }
        if active    { return SF.Colors.purple }
        return SF.Colors.bgTertiary
    }
}

private extension Array {
    subscript(safe index: Int) -> Element? {
        indices.contains(index) ? self[index] : nil
    }
}

// MARK: - Mini Phase Timeline (collapsed card — 14 dots)

struct MiniPhaseTimeline: View {
    let activePhase: Int
    let projectDone: Bool

    private let dotSize: CGFloat = 22
    private let labelWidth: CGFloat = 54
    private let connectorWidth: CGFloat = 10

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 0) {
                ForEach(0..<safePhases.count, id: \.self) { i in
                    HStack(spacing: 0) {
                        phaseDot(index: i)
                        if i < safePhases.count - 1 {
                            phaseConnector(index: i)
                        }
                    }
                }
            }
            .padding(.horizontal, 4)
            .padding(.vertical, 4)
        }
        .frame(height: 50)
    }

    private func phaseDot(index: Int) -> some View {
        let isCompleted = index < activePhase
        let isActive = index == activePhase && !projectDone
        let isDone = projectDone

        return VStack(spacing: 3) {
            ZStack {
                Circle()
                    .fill(dotFill(completed: isCompleted || isDone, active: isActive))
                    .frame(width: dotSize, height: dotSize)

                if isCompleted || isDone {
                    Image(systemName: "checkmark")
                        .font(.system(size: 9, weight: .bold))
                        .foregroundColor(.white)
                } else if isActive {
                    Circle()
                        .stroke(SF.Colors.purple.opacity(0.6), lineWidth: 2)
                        .frame(width: dotSize + 5, height: dotSize + 5)
                    Text("\(index + 1)")
                        .font(.system(size: 9, weight: .bold))
                        .foregroundColor(.white)
                } else {
                    Text("\(index + 1)")
                        .font(.system(size: 9, weight: .semibold))
                        .foregroundColor(SF.Colors.textMuted)
                }
            }

            Text(safePhases[index].short)
                .font(.system(size: 8, weight: .medium))
                .foregroundColor(
                    (isCompleted || isDone) ? SF.Colors.textSecondary :
                    isActive ? SF.Colors.purple :
                    SF.Colors.textMuted.opacity(0.5)
                )
                .lineLimit(1)
                .frame(width: labelWidth)
        }
    }

    private func phaseConnector(index: Int) -> some View {
        let isCompleted = index < activePhase || projectDone
        return Rectangle()
            .fill(isCompleted ? SF.Colors.success.opacity(0.5) : SF.Colors.border.opacity(0.4))
            .frame(width: connectorWidth, height: 2)
            .padding(.bottom, 16)
    }

    private func dotFill(completed: Bool, active: Bool) -> Color {
        if completed { return SF.Colors.success }
        if active    { return SF.Colors.purple }
        return SF.Colors.bgTertiary
    }
}

// MARK: - Color hex-string init

extension Color {
    init(hex: String) {
        let hex = hex.trimmingCharacters(in: CharacterSet.alphanumerics.inverted)
        var int: UInt64 = 0
        Scanner(string: hex).scanHexInt64(&int)
        let r = Double((int >> 16) & 0xFF) / 255.0
        let g = Double((int >> 8) & 0xFF) / 255.0
        let b = Double(int & 0xFF) / 255.0
        self.init(red: r, green: g, blue: b)
    }
}
