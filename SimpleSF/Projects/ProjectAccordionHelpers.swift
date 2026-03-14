import SwiftUI

// Ref: FT-SSF-003

// MARK: - Helper Methods (extension on ProjectAccordion)

extension ProjectAccordion {

    // MARK: - Simulated Phases

    func simulatedPhases() -> [SFBridge.PhaseInfo] {
        safePhases.enumerated().map { i, p in
            SFBridge.PhaseInfo(
                id: "sim-\(i)",
                phase_name: p.name,
                pattern: p.pattern,
                status: i < activePhase ? "completed" : (i == activePhase && isActive ? "running" : "pending"),
                agent_ids: "[]",
                output: nil,
                started_at: nil,
                completed_at: nil,
                phase_type: "once",
                iteration: 1,
                max_iterations: 1
            )
        }
    }

    // MARK: - Messages for Phase

    func messagesForPhase(_ phase: SFBridge.PhaseInfo) -> [SFBridge.MessageInfo] {
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

    // MARK: - Agent Avatar Stack

    func agentAvatarStack(_ agentIdsJson: String) -> some View {
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

    // MARK: - Phase Visual Helpers

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
            .font(.system(size: 9, weight: .semibold))
            .foregroundColor(color)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(color.opacity(0.1))
            .cornerRadius(4)
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

    func messageTypeColor(_ type: String) -> Color {
        switch type {
        case "instruction", "delegation": return SF.Colors.yellowDeep
        case "approval":                  return SF.Colors.success
        case "veto":                      return SF.Colors.error
        case "synthesis":                 return SF.Colors.po
        default:                          return SF.Colors.textMuted
        }
    }
}
