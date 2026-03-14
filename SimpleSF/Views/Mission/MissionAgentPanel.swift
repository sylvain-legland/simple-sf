import SwiftUI

// Ref: FT-SSF-004
// Live events feed, agent panels, and shared helpers for MissionView.

extension MissionView {

    // ── Live events feed (no phase selected) ──

    var liveEventsFeed: some View {
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

    func eventRow(_ event: SFBridge.AgentEvent) -> some View {
        let color = catalog.agentColor(event.agentId)
        let agentRole = catalog.agentRole(event.agentId)
        return HStack(alignment: .top, spacing: 10) {
            AgentAvatarView(agentId: event.agentId, size: 32)
                .overlay(Circle().stroke(color.opacity(0.4), lineWidth: 1.5))
            VStack(alignment: .leading, spacing: 4) {
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

    func messageTypeColor(_ type: String) -> Color {
        switch type {
        case "instruction", "delegation": return SF.Colors.yellowDeep
        case "approval":                  return SF.Colors.success
        case "veto":                      return SF.Colors.error
        case "synthesis":                 return SF.Colors.po
        default:                          return SF.Colors.textMuted
        }
    }

    // MARK: - Shared Helpers

    func phaseCircleFill(status: String) -> Color {
        switch status {
        case "completed": return SF.Colors.success
        case "running":   return SF.Colors.purple
        case "failed":    return SF.Colors.error
        case "vetoed":    return SF.Colors.warning
        default:          return SF.Colors.bgTertiary
        }
    }

    func phaseStatusChip(_ status: String) -> some View {
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

    func patternColor(_ pattern: String) -> Color {
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

    func phaseShortName(_ name: String) -> String {
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

    func agentAvatarStack(_ agentIdsJson: String) -> some View {
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

    func messagesForPhase(_ phase: SFBridge.PhaseInfo) -> [SFBridge.MessageInfo] {
        guard let messages = status?.messages else { return [] }
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

    func eventLabel(_ type: String) -> String {
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
}
