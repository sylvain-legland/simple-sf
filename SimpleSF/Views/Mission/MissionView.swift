import SwiftUI

struct MissionView: View {
    @ObservedObject private var bridge = SFBridge.shared
    @State private var selectedProject: SFBridge.SFProject?
    @State private var brief = ""
    @State private var status: SFBridge.MissionStatus?
    @State private var pollTimer: Timer?

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Image(systemName: "play.circle.fill")
                    .font(.title2)
                    .foregroundColor(.purple)
                Text("Mission Control")
                    .font(.title2.bold())
                Spacer()
                if bridge.isRunning {
                    HStack(spacing: 6) {
                        ProgressView().scaleEffect(0.7)
                        Text("Running...")
                            .font(.caption)
                            .foregroundColor(.orange)
                    }
                }
            }
            .padding()

            Divider()

            if !bridge.isRunning && bridge.currentMissionId == nil {
                launchForm
            } else {
                missionTimeline
            }
        }
        .onAppear { startPolling() }
        .onDisappear { stopPolling() }
    }

    private var launchForm: some View {
        VStack(spacing: 20) {
            Spacer()
            Image(systemName: "rocket.fill")
                .font(.system(size: 56))
                .foregroundColor(.purple.opacity(0.6))

            Text("Start a new mission")
                .font(.title3.bold())
            Text("Describe what you want to build. The agent team will handle vision, design, development, QA, and review.")
                .font(.callout)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 500)

            let projects = bridge.listProjects()
            if !projects.isEmpty {
                Picker("Project", selection: $selectedProject) {
                    Text("Select a project...").tag(nil as SFBridge.SFProject?)
                    ForEach(projects) { p in
                        Text(p.name).tag(p as SFBridge.SFProject?)
                    }
                }
                .frame(maxWidth: 400)
            }

            TextEditor(text: $brief)
                .font(.body.monospaced())
                .frame(maxWidth: 500, minHeight: 120, maxHeight: 120)
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(Color.purple.opacity(0.3), lineWidth: 1)
                )

            Button(action: launchMission) {
                Label("Launch Mission", systemImage: "play.fill")
                    .font(.headline)
            }
            .buttonStyle(.borderedProminent)
            .tint(.purple)
            .disabled(brief.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)

            Spacer()
        }
        .padding()
    }

    private var missionTimeline: some View {
        HSplitView {
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 12) {
                        if let phases = status?.phases {
                            phasesSection(phases)
                        }

                        Divider().padding(.vertical, 8)

                        ForEach(bridge.events) { event in
                            eventRow(event).id(event.id)
                        }
                    }
                    .padding()
                }
                .onChange(of: bridge.events.count) { _, _ in
                    if let last = bridge.events.last {
                        withAnimation { proxy.scrollTo(last.id, anchor: .bottom) }
                    }
                }
            }
            .frame(minWidth: 400)

            ScrollView {
                LazyVStack(alignment: .leading, spacing: 8) {
                    if let messages = status?.messages {
                        ForEach(messages.reversed()) { msg in
                            messageRow(msg)
                        }
                    }
                }
                .padding()
            }
            .frame(minWidth: 300)
        }
    }

    private func phasesSection(_ phases: [SFBridge.PhaseInfo]) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("SAFe Workflow").font(.headline)
            HStack(spacing: 8) {
                ForEach(phases) { phase in
                    phaseChip(phase)
                }
            }
        }
    }

    private func phaseChip(_ phase: SFBridge.PhaseInfo) -> some View {
        HStack(spacing: 4) {
            phaseIcon(phase.status)
            Text(phase.phase_name.capitalized)
                .font(.caption.bold())
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .background(phaseColor(phase.status).opacity(0.15))
        .foregroundColor(phaseColor(phase.status))
        .cornerRadius(12)
    }

    @ViewBuilder
    private func phaseIcon(_ status: String) -> some View {
        switch status {
        case "completed":
            Image(systemName: "checkmark.circle.fill").font(.caption)
        case "running":
            ProgressView().scaleEffect(0.5)
        case "failed":
            Image(systemName: "xmark.circle.fill").font(.caption)
        default:
            Image(systemName: "circle").font(.caption)
        }
    }

    private func phaseColor(_ status: String) -> Color {
        switch status {
        case "completed": return .green
        case "running":   return .orange
        case "failed":    return .red
        default:          return .gray
        }
    }

    private func eventRow(_ event: SFBridge.AgentEvent) -> some View {
        HStack(alignment: .top, spacing: 8) {
            agentAvatar(event.agentId)
            VStack(alignment: .leading, spacing: 2) {
                HStack {
                    Text(event.agentId)
                        .font(.caption.bold())
                        .foregroundColor(.purple)
                    Text(eventLabel(event.eventType))
                        .font(.caption2)
                        .foregroundColor(.secondary)
                    Spacer()
                    Text(event.timestamp, style: .time)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
                if !event.data.isEmpty && event.eventType != "thinking" {
                    Text(String(event.data.prefix(500)))
                        .font(.caption.monospaced())
                        .foregroundColor(eventColor(event.eventType))
                        .lineLimit(6)
                }
            }
        }
        .padding(8)
        .background(Color.gray.opacity(0.05))
        .cornerRadius(8)
    }

    private func agentAvatar(_ agentId: String) -> some View {
        let initial = String(agentId.split(separator: "-").last?.prefix(1).uppercased() ?? "?")
        return Text(initial)
            .font(.caption2.bold())
            .frame(width: 24, height: 24)
            .background(agentColor(agentId))
            .foregroundColor(.white)
            .clipShape(Circle())
    }

    private func agentColor(_ agentId: String) -> Color {
        if agentId.contains("rte") { return .blue }
        if agentId.contains("po") { return .green }
        if agentId.contains("lead") { return .orange }
        if agentId.contains("dev") { return .purple }
        if agentId.contains("qa") { return .red }
        return .gray
    }

    private func eventLabel(_ type: String) -> String {
        switch type {
        case "thinking":         return "is thinking..."
        case "tool_call":        return "called a tool"
        case "tool_result":      return "got result"
        case "response":         return "responded"
        case "error":            return "error"
        case "mission_complete": return "mission complete"
        default:                 return type
        }
    }

    private func eventColor(_ type: String) -> Color {
        switch type {
        case "tool_call":   return .blue
        case "tool_result": return .cyan
        case "response":    return .primary
        case "error":       return .red
        default:            return .secondary
        }
    }

    private func messageRow(_ msg: SFBridge.MessageInfo) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(msg.agent_name)
                    .font(.caption.bold())
                    .foregroundColor(.purple)
                Text("(\(msg.role))")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
            Text(String(msg.content.prefix(300)))
                .font(.caption)
        }
        .padding(8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.gray.opacity(0.05))
        .cornerRadius(6)
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
