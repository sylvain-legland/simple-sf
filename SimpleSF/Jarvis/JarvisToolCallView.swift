import SwiftUI

// Ref: FT-SSF-001
// Agent message cards, phase headers, and thinking indicators for JarvisView.

extension JarvisView {

    // MARK: - Phase header (shown before first agent message group)

    var phaseHeader: some View {
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
    func agentMessageCardFromStored(_ msg: LLMMessage) -> some View {
        let aid = msg.agentId ?? "engine"
        let name = msg.agentName ?? catalog.agentName(aid)
        let role = msg.agentRole ?? catalog.agentRole(aid)
        let mtype = msg.messageType ?? "response"
        let recipients = msg.toAgents ?? []
        let roleColor = catalog.agentColor(aid)
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

    // ── Thinking indicator ──

    @ViewBuilder
    func thinkingIndicator(event: SFBridge.AgentEvent) -> some View {
        let name = event.agentName.isEmpty
            ? catalog.agentName(event.agentId)
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

    // MARK: - Session Sidebar

    var sessionSidebar: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Historique")
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundColor(SF.Colors.textSecondary)
                Spacer()
                Button(action: {
                    chatStore.newSession()
                    bridge.discussionEvents.removeAll()
                }) {
                    Image(systemName: "square.and.pencil")
                        .font(.system(size: 14))
                        .foregroundColor(SF.Colors.purple)
                }
                .buttonStyle(.plain)
                .help(L10n.shared.t(.jarvisNewConversation))
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

    var emptyState: some View {
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

    // MARK: - Private helpers

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
                let displayName = agentId == "all" ? "Tous" : catalog.agentName(agentId)
                let color = catalog.agentColor(agentId)
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

    // ── Helper: border color by message type (SF legacy) ──

    private func borderColorFor(_ messageType: String) -> Color {
        switch messageType {
        case "instruction", "request", "delegation":
            return Color(red: 0.92, green: 0.70, blue: 0.03)
        case "response", "approval":
            return Color(red: 0.13, green: 0.77, blue: 0.37)
        case "veto":
            return Color(red: 0.94, green: 0.27, blue: 0.27)
        case "synthesis":
            return SF.Colors.po
        default:
            return SF.Colors.textMuted.opacity(0.5)
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

    func formatTime(_ date: Date) -> String {
        let fmt = DateFormatter()
        fmt.dateFormat = "HH:mm"
        return fmt.string(from: date)
    }
}
