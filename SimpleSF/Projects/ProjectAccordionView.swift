import SwiftUI

// Ref: FT-SSF-003

// MARK: - Project Accordion (card header + collapsible discussion panel)

@MainActor
struct ProjectAccordion: View {
    let project: Project
    let isExpanded: Bool
    let toggle: () -> Void

    @ObservedObject var bridge = SFBridge.shared
    @ObservedObject var catalog = SFCatalog.shared
    @State var missionStatus: SFBridge.MissionStatus?
    @State var selectedPhaseIndex: Int?
    @State var pollTimer: Timer?

    /// Events scoped to this project (not the global feed)
    var projectEvents: [SFBridge.AgentEvent] {
        bridge.eventsForProject(project.id)
    }

    var activePhase: Int {
        if let real = missionStatus?.phases, !real.isEmpty {
            let completed = real.filter { $0.status == "completed" || $0.status == "approved" }.count
            let hasRunning = real.contains { $0.status == "running" }
            return hasRunning ? completed : completed
        }
        return simulatedActivePhase(for: project.status, progress: project.progress)
    }

    var isActive: Bool { project.status == .active }
    var isPaused: Bool { project.status == .paused }
    var isQueued: Bool { project.status == .planning }
    var isDone: Bool { project.status == .done }

    /// Phases to display: real from mission status, or simulated fallback
    var displayPhases: [SFBridge.PhaseInfo] {
        if let real = missionStatus?.phases, !real.isEmpty { return real }
        return simulatedPhases()
    }

    /// True when a workflow/mission has been started (or project has progressed beyond idea)
    var hasWorkflow: Bool {
        project.missionId != nil || project.status != .idea
    }

    /// Current phase pattern (for display in event cards)
    var currentPhasePattern: String? {
        let phases = displayPhases
        if let running = phases.first(where: { $0.status == "running" }) { return running.pattern }
        return nil
    }

    var body: some View {
        VStack(spacing: 0) {
            cardHeader
                .contentShape(Rectangle())
                .simultaneousGesture(TapGesture().onEnded { toggle() })

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
                        Text("\(activePhase)/\(displayPhases.count) phases")
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
                                .frame(width: geo.size.width * CGFloat(activePhase) / CGFloat(max(displayPhases.count, 1)), height: 6)
                        }
                    }
                    .frame(height: 6)
                }
                .padding(.leading, 24)

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
    var conversationFeed: some View {
        if !projectEvents.isEmpty {
            liveEventsFeed
        } else if isActive && !bridge.events.isEmpty {
            globalEventsFeed
        } else if isActive && !bridge.discussionEvents.isEmpty {
            eventScrollFeed(events: bridge.discussionEvents)
        } else if let msgs = missionStatus?.messages, !msgs.isEmpty {
            missionMessagesFeed(msgs)
        } else {
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

    // MARK: - Status Indicator

    @ViewBuilder
    var statusIndicator: some View {
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

    // MARK: - Control Buttons

    var controlButtons: some View {
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

    // MARK: - Actions

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

    func startPolling() {
        pollTimer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { _ in
            Task { @MainActor in
                self.missionStatus = bridge.missionStatusForProject(project.id)
            }
        }
    }

    func stopPolling() {
        pollTimer?.invalidate()
        pollTimer = nil
    }
}
