import SwiftUI

// Ref: FT-SSF-004
// Phase timeline, phase nodes, and phase detail panel for MissionView.

extension MissionView {

    // ── Horizontal phase timeline ──

    var phaseTimeline: some View {
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

    func phaseNode(index: Int, phase: SFBridge.PhaseInfo) -> some View {
        let isSelected = selectedPhaseIndex == index
        let isActive = phase.status == "running"
        let isDone = phase.status == "completed"
        let isFailed = phase.status == "failed" || phase.status == "vetoed"

        return Button(action: { withAnimation(.easeInOut(duration: 0.2)) { selectedPhaseIndex = index } }) {
            VStack(spacing: 6) {
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

                Text(phaseShortName(phase.phase_name))
                    .font(.system(size: 10, weight: isSelected ? .bold : .medium))
                    .foregroundColor(isSelected ? SF.Colors.purple : (isDone ? SF.Colors.textSecondary : SF.Colors.textMuted))
                    .lineLimit(2)
                    .multilineTextAlignment(.center)
                    .frame(width: 72)

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

    func phaseConnector(done: Bool) -> some View {
        Rectangle()
            .fill(done ? SF.Colors.success.opacity(0.5) : SF.Colors.border)
            .frame(width: 20, height: 2)
            .padding(.bottom, 30)
    }

    // ── Phase detail + agent feed ──

    @ViewBuilder
    var phaseDetailOrFeed: some View {
        let phases = status?.phases ?? []
        if let idx = selectedPhaseIndex, idx < phases.count {
            phaseDetailPanel(phases[idx], index: idx)
        } else {
            liveEventsFeed
        }
    }

    func phaseDetailPanel(_ phase: SFBridge.PhaseInfo, index: Int) -> some View {
        VStack(spacing: 0) {
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

            ScrollView {
                LazyVStack(alignment: .leading, spacing: 10) {
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

    func phaseMessageCard(_ msg: SFBridge.MessageInfo, pattern: String, phaseAgentIds: String) -> some View {
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

            HStack(alignment: .top, spacing: 12) {
                AgentAvatarView(agentId: aid, size: 40)
                    .overlay(Circle().stroke(color.opacity(0.5), lineWidth: 2))

                VStack(alignment: .leading, spacing: 6) {
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
}
