import SwiftUI

// Ref: FT-SSF-003

// MARK: - Message & Discussion Feed Views (extension on ProjectAccordion)

extension ProjectAccordion {

    // MARK: - Phase Detail Panel

    func phaseDetailPanel(_ phase: SFBridge.PhaseInfo, index: Int) -> some View {
        VStack(spacing: 0) {
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

    // MARK: - Phase Message Card

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

    // MARK: - Mission Messages Feed

    func missionMessagesFeed(_ messages: [SFBridge.MessageInfo]) -> some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 8) {
                ForEach(Array(messages.enumerated()), id: \.offset) { _, msg in
                    missionMessageRow(msg)
                }
            }
            .padding(16)
        }
    }

    func missionMessageRow(_ msg: SFBridge.MessageInfo) -> some View {
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

    // MARK: - Discussion Messages Feed

    func discussionMessagesFeed(_ messages: [SFBridge.DiscussionMessage]) -> some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 8) {
                ForEach(messages) { msg in
                    discussionMessageRow(msg)
                }
            }
            .padding(16)
        }
    }

    func discussionMessageRow(_ msg: SFBridge.DiscussionMessage) -> some View {
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

    // MARK: - Placeholders

    var emptyDiscussionPlaceholder: some View {
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

    var activeNoDataPlaceholder: some View {
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
}
