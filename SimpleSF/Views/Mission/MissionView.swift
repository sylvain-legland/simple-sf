import SwiftUI

// MARK: - Value Stream View (SF Legacy "Value Stream - Epics")
// Horizontal phase timeline per epic, click-to-drill agent discussions.
// Phases from product-lifecycle workflow: 14 phases, each with pattern + gate.

// Ref: FT-SSF-004
struct MissionView: View {
    @ObservedObject private var bridge = SFBridge.shared
    @ObservedObject private var catalog = SFCatalog.shared
    @State private var selectedProject: SFBridge.SFProject?
    @State private var brief = ""
    @State private var status: SFBridge.MissionStatus?
    @State private var selectedPhaseIndex: Int?
    @State private var pollTimer: Timer?

    var body: some View {
        VStack(spacing: 0) {
            if !bridge.isRunning && bridge.currentMissionId == nil {
                launchForm
            } else {
                valueStreamView
            }
        }
        .background(SF.Colors.bgPrimary)
        .onAppear { startPolling() }
        .onDisappear { stopPolling() }
    }

    // MARK: - Launch Form

    private var launchForm: some View {
        VStack(spacing: 0) {
            // Header
            HStack(spacing: 10) {
                Image(systemName: "flowchart.fill")
                    .font(.system(size: 20))
                    .foregroundColor(SF.Colors.purple)
                Text("Value Stream")
                    .font(.system(size: 22, weight: .bold))
                    .foregroundColor(SF.Colors.textPrimary)
                Spacer()
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 16)

            Divider().background(SF.Colors.border)

            Spacer()

            VStack(spacing: 24) {
                Image(systemName: "rocket.fill")
                    .font(.system(size: 52))
                    .foregroundColor(SF.Colors.purple.opacity(0.5))

                Text("Lancer un Epic")
                    .font(.system(size: 20, weight: .bold))
                    .foregroundColor(SF.Colors.textPrimary)

                Text("Décrivez votre produit. L'équipe SAFe enchaîne 14 phases :\nIdéation → Comité Stratégique → Architecture → Sprints Dev → QA → Deploy Prod → TMA")
                    .font(.system(size: 13))
                    .foregroundColor(SF.Colors.textSecondary)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 600)

                let projects = bridge.listProjects()
                if !projects.isEmpty {
                    Picker("Projet", selection: $selectedProject) {
                        Text("Sélectionner un projet…").tag(nil as SFBridge.SFProject?)
                        ForEach(projects) { p in
                            Text(p.name).tag(p as SFBridge.SFProject?)
                        }
                    }
                    .frame(maxWidth: 400)
                }

                TextEditor(text: $brief)
                    .font(.system(size: 13).monospaced())
                    .foregroundColor(SF.Colors.textPrimary)
                    .scrollContentBackground(.hidden)
                    .frame(maxWidth: 600, minHeight: 100, maxHeight: 120)
                    .padding(12)
                    .background(SF.Colors.bgTertiary)
                    .cornerRadius(10)
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .stroke(SF.Colors.border, lineWidth: 1)
                    )

                Button(action: launchMission) {
                    HStack(spacing: 8) {
                        Image(systemName: "play.fill")
                        Text("Lancer le Workflow SAFe")
                            .font(.system(size: 14, weight: .semibold))
                    }
                    .padding(.horizontal, 24)
                    .padding(.vertical, 10)
                    .background(SF.Colors.purple)
                    .foregroundColor(.white)
                    .cornerRadius(10)
                }
                .buttonStyle(.plain)
                .disabled(brief.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }

            Spacer()
        }
    }

    // MARK: - Value Stream (Epic Timeline + Phase Detail)

    private var valueStreamView: some View {
        VStack(spacing: 0) {
            epicHeader
            Divider().background(SF.Colors.border)
            phaseTimeline
            Divider().background(SF.Colors.border)
            phaseDetailOrFeed
        }
    }

    // ── Epic header banner ──

    private var epicHeader: some View {
        HStack(spacing: 14) {
            Image(systemName: "flowchart.fill")
                .font(.system(size: 18))
                .foregroundColor(SF.Colors.purple)

            VStack(alignment: .leading, spacing: 2) {
                Text("Cycle de Vie Produit Complet")
                    .font(.system(size: 16, weight: .bold))
                    .foregroundColor(SF.Colors.textPrimary)
                Text(status?.mission?.brief.prefix(120) ?? brief.prefix(120))
                    .font(.system(size: 12))
                    .foregroundColor(SF.Colors.textSecondary)
                    .lineLimit(1)
            }

            Spacer()

            missionStatusBadge
        }
        .padding(.horizontal, 24)
        .padding(.vertical, 14)
        .background(SF.Colors.bgSecondary)
    }

    @ViewBuilder
    private var missionStatusBadge: some View {
        let s = status?.mission?.status ?? "running"
        let (label, color, icon): (String, Color, String) = {
            switch s {
            case "completed": return ("Terminé", SF.Colors.success, "checkmark.circle.fill")
            case "failed":    return ("Échoué", SF.Colors.error, "xmark.circle.fill")
            case "vetoed":    return ("Véto", SF.Colors.warning, "exclamationmark.triangle.fill")
            default:          return ("En cours", SF.Colors.purple, "play.circle.fill")
            }
        }()
        HStack(spacing: 6) {
            if s == "running" { ProgressView().scaleEffect(0.6) }
            Image(systemName: icon).font(.system(size: 12))
            Text(label).font(.system(size: 12, weight: .semibold))
        }
        .foregroundColor(color)
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .background(color.opacity(0.12))
        .cornerRadius(8)
    }

    // ── Horizontal phase timeline ──

    private var phaseTimeline: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 0) {
                let phases = status?.phases ?? []
                ForEach(Array(phases.enumerated()), id: \.element.id) { index, phase in
                    HStack(spacing: 0) {
                        phaseNode(index: index, phase: phase)
                        if index < phases.count - 1 {
                            phaseConnector(done: phase.status == "completed")
                        }
                    }
                }
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 16)
        }
        .frame(height: 110)
        .background(SF.Colors.bgSecondary.opacity(0.5))
    }

    private func phaseNode(index: Int, phase: SFBridge.PhaseInfo) -> some View {
        let isSelected = selectedPhaseIndex == index
        let isActive = phase.status == "running"
        let isDone = phase.status == "completed"
        let isFailed = phase.status == "failed" || phase.status == "vetoed"

        return Button(action: { withAnimation(.easeInOut(duration: 0.2)) { selectedPhaseIndex = index } }) {
            VStack(spacing: 6) {
                // Numbered circle
                ZStack {
                    Circle()
                        .fill(phaseCircleFill(status: phase.status))
                        .frame(width: 36, height: 36)
                    if isActive {
                        Circle()
                            .stroke(SF.Colors.purple, lineWidth: 2)
                            .frame(width: 42, height: 42)
                            .opacity(0.6)
                    }
                    if isDone {
                        Image(systemName: "checkmark")
                            .font(.system(size: 14, weight: .bold))
                            .foregroundColor(.white)
                    } else if isFailed {
                        Image(systemName: "xmark")
                            .font(.system(size: 14, weight: .bold))
                            .foregroundColor(.white)
                    } else if isActive {
                        ProgressView()
                            .scaleEffect(0.55)
                            .tint(.white)
                    } else {
                        Text("\(index + 1)")
                            .font(.system(size: 13, weight: .bold))
                            .foregroundColor(SF.Colors.textMuted)
                    }
                }

                // Phase name
                Text(phaseShortName(phase.phase_name))
                    .font(.system(size: 10, weight: isSelected ? .bold : .medium))
                    .foregroundColor(isSelected ? SF.Colors.purple : (isDone ? SF.Colors.textSecondary : SF.Colors.textMuted))
                    .lineLimit(2)
                    .multilineTextAlignment(.center)
                    .frame(width: 72)

                // Pattern badge
                Text(phase.pattern)
                    .font(.system(size: 8, weight: .medium))
                    .foregroundColor(patternColor(phase.pattern))
                    .padding(.horizontal, 5)
                    .padding(.vertical, 2)
                    .background(patternColor(phase.pattern).opacity(0.1))
                    .cornerRadius(4)
            }
        }
        .buttonStyle(.plain)
        .padding(.horizontal, 4)
        .padding(.vertical, 6)
        .background(isSelected ? SF.Colors.purple.opacity(0.08) : Color.clear)
        .cornerRadius(8)
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(isSelected ? SF.Colors.purple.opacity(0.4) : Color.clear, lineWidth: 1)
        )
    }

    private func phaseConnector(done: Bool) -> some View {
        Rectangle()
            .fill(done ? SF.Colors.success.opacity(0.5) : SF.Colors.border)
            .frame(width: 20, height: 2)
            .padding(.bottom, 30)
    }

    // ── Phase detail + agent feed ──

    @ViewBuilder
    private var phaseDetailOrFeed: some View {
        let phases = status?.phases ?? []
        if let idx = selectedPhaseIndex, idx < phases.count {
            phaseDetailPanel(phases[idx], index: idx)
        } else {
            liveEventsFeed
        }
    }

    private func phaseDetailPanel(_ phase: SFBridge.PhaseInfo, index: Int) -> some View {
        VStack(spacing: 0) {
            // Phase detail header
            HStack(spacing: 12) {
                ZStack {
                    Circle()
                        .fill(phaseCircleFill(status: phase.status))
                        .frame(width: 32, height: 32)
                    if phase.status == "completed" {
                        Image(systemName: "checkmark").font(.system(size: 12, weight: .bold)).foregroundColor(.white)
                    } else {
                        Text("\(index + 1)").font(.system(size: 12, weight: .bold)).foregroundColor(.white)
                    }
                }

                VStack(alignment: .leading, spacing: 2) {
                    Text(phase.phase_name)
                        .font(.system(size: 16, weight: .bold))
                        .foregroundColor(SF.Colors.textPrimary)
                    HStack(spacing: 8) {
                        PatternBadge(pattern: phase.pattern)
                        phaseStatusChip(phase.status)
                        if let started = phase.started_at {
                            Text(started.prefix(16))
                                .font(.system(size: 11))
                                .foregroundColor(SF.Colors.textMuted)
                        }
                    }
                }

                Spacer()

                // Agent avatars for this phase
                agentAvatarStack(phase.agent_ids)

                Button(action: { selectedPhaseIndex = nil }) {
                    Image(systemName: "xmark.circle.fill")
                        .font(.system(size: 16))
                        .foregroundColor(SF.Colors.textMuted)
                }
                .buttonStyle(.plain)
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 12)
            .background(SF.Colors.bgSecondary)

            Divider().background(SF.Colors.border)

            // Phase messages
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 10) {
                    // Filter messages for this phase
                    let phaseMessages = messagesForPhase(phase)
                    if phaseMessages.isEmpty {
                        HStack {
                            Spacer()
                            VStack(spacing: 8) {
                                Image(systemName: phase.status == "pending" ? "clock" : "bubble.left.and.bubble.right")
                                    .font(.system(size: 28))
                                    .foregroundColor(SF.Colors.textMuted)
                                Text(phase.status == "pending" ? "Phase en attente" : "Discussion en cours…")
                                    .font(.system(size: 13))
                                    .foregroundColor(SF.Colors.textMuted)
                            }
                            .padding(.top, 40)
                            Spacer()
                        }
                    } else {
                        ForEach(phaseMessages) { msg in
                            phaseMessageCard(msg, pattern: phase.pattern, phaseAgentIds: phase.agent_ids)
                        }
                    }

                    // Phase output (summary)
                    if let output = phase.output, !output.isEmpty {
                        VStack(alignment: .leading, spacing: 8) {
                            HStack(spacing: 6) {
                                Image(systemName: "doc.text")
                                    .font(.system(size: 12))
                                    .foregroundColor(SF.Colors.textSecondary)
                                Text("Résultat de phase")
                                    .font(.system(size: 12, weight: .semibold))
                                    .foregroundColor(SF.Colors.textSecondary)
                            }
                            MarkdownView(output, fontSize: 13)
                                .textSelection(.enabled)
                        }
                        .padding(16)
                        .background(SF.Colors.bgTertiary)
                        .cornerRadius(10)
                    }
                }
                .padding(20)
            }
        }
    }

    private func phaseMessageCard(_ msg: SFBridge.MessageInfo, pattern: String, phaseAgentIds: String) -> some View {
        let aid = msg.role
        let color = catalog.agentColor(aid)
        let agentRole = catalog.agentRole(aid)
        // Parse phase agent IDs to show recipients (other agents in the phase)
        let recipients: [String] = {
            guard let data = phaseAgentIds.data(using: .utf8),
                  let arr = try? JSONDecoder().decode([String].self, from: data) else { return [] }
            return arr.filter { $0 != aid }
        }()

        return HStack(alignment: .top, spacing: 0) {
            RoundedRectangle(cornerRadius: 2)
                .fill(color)
                .frame(width: 3)

            HStack(alignment: .top, spacing: 12) {
                AgentAvatarView(agentId: aid, size: 40)
                    .overlay(Circle().stroke(color.opacity(0.5), lineWidth: 2))

                VStack(alignment: .leading, spacing: 6) {
                    // Name + Role + Pattern
                    HStack(spacing: 6) {
                        Text(msg.agent_name)
                            .font(.system(size: 14, weight: .bold))
                            .foregroundColor(color)
                        if !agentRole.isEmpty {
                            Text(agentRole)
                                .font(.system(size: 10, weight: .medium))
                                .foregroundColor(SF.Colors.textSecondary)
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(color.opacity(0.1))
                                .cornerRadius(4)
                        }
                        PatternBadge(pattern: pattern)
                        Spacer()
                        Text(msg.created_at.suffix(8))
                            .font(.system(size: 10))
                            .foregroundColor(SF.Colors.textMuted)
                    }

                    // Recipients
                    if !recipients.isEmpty {
                        HStack(spacing: 4) {
                            Image(systemName: "arrow.right")
                                .font(.system(size: 9))
                                .foregroundColor(SF.Colors.textMuted)
                            ForEach(recipients.prefix(4), id: \.self) { rid in
                                HStack(spacing: 3) {
                                    AgentAvatarView(agentId: rid, size: 16)
                                    Text(catalog.agentName(rid))
                                        .font(.system(size: 10))
                                        .foregroundColor(SF.Colors.textSecondary)
                                }
                            }
                            if recipients.count > 4 {
                                Text("+\(recipients.count - 4)")
                                    .font(.system(size: 10))
                                    .foregroundColor(SF.Colors.textMuted)
                            }
                        }
                    }

                    MarkdownView(msg.content, fontSize: 13)
                        .textSelection(.enabled)
                }
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 12)
        }
        .background(SF.Colors.bgCard)
        .cornerRadius(10)
        .overlay(
            RoundedRectangle(cornerRadius: 10)
                .stroke(SF.Colors.border.opacity(0.4), lineWidth: 0.5)
        )
    }

    // ── Live events feed (no phase selected) ──

    private var liveEventsFeed: some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 8) {
                    ForEach(bridge.events) { event in
                        eventRow(event).id(event.id)
                    }
                }
                .padding(20)
            }
            .onChange(of: bridge.events.count) { _, _ in
                if let last = bridge.events.last {
                    withAnimation { proxy.scrollTo(last.id, anchor: .bottom) }
                }
            }
        }
    }

    private func eventRow(_ event: SFBridge.AgentEvent) -> some View {
        let color = catalog.agentColor(event.agentId)
        let agentRole = catalog.agentRole(event.agentId)
        return HStack(alignment: .top, spacing: 10) {
            AgentAvatarView(agentId: event.agentId, size: 32)
                .overlay(Circle().stroke(color.opacity(0.4), lineWidth: 1.5))
            VStack(alignment: .leading, spacing: 4) {
                // Name + role + event type
                HStack(spacing: 6) {
                    Text(catalog.agentName(event.agentId))
                        .font(.system(size: 12, weight: .semibold))
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
                    if !event.messageType.isEmpty && event.messageType != "response" {
                        Text(event.messageType)
                            .font(.system(size: 9, weight: .bold))
                            .foregroundColor(.white)
                            .padding(.horizontal, 5)
                            .padding(.vertical, 1)
                            .background(messageTypeColor(event.messageType))
                            .cornerRadius(3)
                    }
                    Spacer()
                    Text(event.timestamp, style: .time)
                        .font(.system(size: 10))
                        .foregroundColor(SF.Colors.textMuted)
                }
                // Recipients
                if !event.toAgents.isEmpty {
                    HStack(spacing: 4) {
                        Image(systemName: "arrow.right")
                            .font(.system(size: 8))
                            .foregroundColor(SF.Colors.textMuted)
                        ForEach(event.toAgents.prefix(3), id: \.self) { rid in
                            Text(catalog.agentName(rid))
                                .font(.system(size: 10))
                                .foregroundColor(SF.Colors.textSecondary)
                        }
                        if event.toAgents.count > 3 {
                            Text("+\(event.toAgents.count - 3)")
                                .font(.system(size: 9))
                                .foregroundColor(SF.Colors.textMuted)
                        }
                    }
                }
                if !event.data.isEmpty && event.eventType != "thinking" {
                    MarkdownView(String(event.data.prefix(600)), fontSize: 12)
                }
            }
        }
        .padding(10)
        .background(SF.Colors.bgCard)
        .cornerRadius(8)
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

    // MARK: - Helpers

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
            .font(.system(size: 10, weight: .semibold))
            .foregroundColor(color)
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(color.opacity(0.1))
            .cornerRadius(6)
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
        // Shorten long phase names for the timeline
        let map: [String: String] = [
            "ideation": "Idéation",
            "strategic-committee": "Comité Strat.",
            "project-setup": "Constitution",
            "architecture": "Architecture",
            "design-system": "Design Sys.",
            "dev-sprint": "Sprints Dev",
            "build-verify": "Build & Verify",
            "cicd": "Pipeline CI",
            "ux-review": "Revue UX",
            "qa-campaign": "Campagne QA",
            "qa-execution": "Exécution",
            "deploy-prod": "Deploy Prod",
            "tma-router": "Routage",
            "tma-fix": "Correctif",
        ]
        return map[name] ?? name.replacingOccurrences(of: "-", with: " ").capitalized
    }

    private func agentAvatarStack(_ agentIdsJson: String) -> some View {
        let ids: [String] = {
            guard let data = agentIdsJson.data(using: .utf8),
                  let arr = try? JSONDecoder().decode([String].self, from: data) else { return [] }
            return arr
        }()
        return HStack(spacing: -8) {
            ForEach(ids.prefix(5), id: \.self) { aid in
                AgentAvatarView(agentId: aid, size: 28)
                    .overlay(Circle().stroke(SF.Colors.bgSecondary, lineWidth: 2))
            }
            if ids.count > 5 {
                Text("+\(ids.count - 5)")
                    .font(.system(size: 10, weight: .bold))
                    .foregroundColor(SF.Colors.textSecondary)
                    .frame(width: 28, height: 28)
                    .background(SF.Colors.bgTertiary)
                    .clipShape(Circle())
            }
        }
    }

    private func messagesForPhase(_ phase: SFBridge.PhaseInfo) -> [SFBridge.MessageInfo] {
        guard let messages = status?.messages else { return [] }
        // Filter by agent IDs in this phase
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

    private func eventLabel(_ type: String) -> String {
        switch type {
        case "thinking":         return "réfléchit…"
        case "tool_call":        return "utilise un outil"
        case "tool_result":      return "résultat"
        case "response":         return "a répondu"
        case "error":            return "erreur"
        case "mission_complete": return "mission terminée"
        default:                 return type
        }
    }

    private func launchMission() {
        let projectId = selectedProject?.id ?? "default"
        let _ = bridge.startMission(projectId: projectId, brief: brief)
        brief = ""
    }

    private func startPolling() {
        pollTimer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { _ in
            Task { @MainActor in
                self.status = bridge.missionStatus()
            }
        }
    }

    private func stopPolling() {
        pollTimer?.invalidate()
        pollTimer = nil
    }
}
