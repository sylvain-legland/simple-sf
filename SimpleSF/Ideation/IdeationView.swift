import SwiftUI

// Ref: FT-SSF-006
@MainActor
struct IdeationView: View {
    @ObservedObject private var bridge = SFBridge.shared
    @ObservedObject private var llm = LLMService.shared

    @State private var idea = ""
    @State private var errorMessage: String?

    private var agentNames: [String: (name: String, icon: String, color: Color)] {
        [
            "ideation-pm":   ("Product Manager", "chart.bar.fill", .blue),
            "ideation-tech": ("Tech Lead", "chevron.left.forwardslash.chevron.right", .green),
            "ideation-ux":   ("UX Designer", "paintbrush.fill", .orange),
            "engine":        ("Orchestrator", "cpu", .purple),
        ]
    }

    var body: some View {
        VStack(spacing: 0) {
            IHMContextHeader(context: .ideation)

            // Header
            HStack {
                Image(systemName: "lightbulb.fill")
                    .foregroundColor(.yellow)
                Text("Ideation")
                    .font(.title2.bold())
                Spacer()
                if bridge.ideationRunning {
                    HStack(spacing: 6) {
                        ProgressView().scaleEffect(0.7)
                        Text("Agents discussing...")
                            .font(.caption)
                            .foregroundColor(.orange)
                    }
                }
            }
            .padding()

            Divider()

            ScrollView {
                VStack(spacing: 16) {
                    // Idea input
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Describe your idea")
                            .font(.headline)
                        TextEditor(text: $idea)
                            .font(.body)
                            .frame(minHeight: 80)
                            .padding(8)
                            .background(Color(.controlBackgroundColor))
                            .cornerRadius(8)
                        if let err = errorMessage {
                            Text(err).foregroundColor(.red).font(.caption)
                        }
                        HStack {
                            Button(action: startIdeation) {
                                Label("Launch Ideation", systemImage: "play.circle.fill")
                            }
                            .buttonStyle(.borderedProminent)
                            .tint(.purple)
                            .disabled(idea.isEmpty || bridge.ideationRunning || llm.activeProvider == nil)

                            if llm.activeProvider == nil {
                                Label("Configure a provider in Settings", systemImage: "exclamationmark.triangle")
                                    .font(.caption)
                                    .foregroundColor(.orange)
                            } else {
                                HStack(spacing: 4) {
                                    Text("3 agents")
                                    Image(systemName: "arrow.triangle.2.circlepath")
                                    Text("3 rounds")
                                    Image(systemName: "arrow.right")
                                    Text("network pattern")
                                }
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            }
                        }
                    }
                    .padding()
                    .background(Color(.controlBackgroundColor))
                    .cornerRadius(10)

                    // Discussion timeline
                    if !bridge.ideationEvents.isEmpty {
                        discussionTimeline
                    }
                }
                .padding()
            }
        }
    }

    // MARK: - Discussion timeline

    private var discussionTimeline: some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(bridge.ideationEvents) { event in
                let info = agentNames[event.agentId] ?? (event.agentId, "person.fill", Color.gray)

                if event.agentId == "engine" {
                    // Round separator
                    HStack {
                        VStack { Divider() }
                        Text(event.data)
                            .font(.caption.bold())
                            .foregroundColor(.purple)
                            .fixedSize()
                        VStack { Divider() }
                    }
                    .padding(.vertical, 8)
                } else {
                    // Agent response
                    HStack(alignment: .top, spacing: 12) {
                        VStack {
                            Image(systemName: info.icon)
                                .foregroundColor(info.color)
                                .font(.system(size: 16))
                                .frame(width: 32, height: 32)
                                .background(info.color.opacity(0.15))
                                .clipShape(Circle())
                        }

                        VStack(alignment: .leading, spacing: 4) {
                            Text(info.name)
                                .font(.subheadline.bold())
                                .foregroundColor(info.color)
                            Text(event.data)
                                .font(.body)
                                .textSelection(.enabled)
                        }
                    }
                    .padding(.vertical, 8)
                    .padding(.horizontal, 4)
                }
            }

            // Completion indicator
            if !bridge.ideationRunning && bridge.ideationEvents.count > 3 {
                HStack {
                    Spacer()
                    VStack(spacing: 4) {
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundColor(.green)
                            .font(.title2)
                        Text("Ideation complete — 3 perspectives, 3 rounds")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    Spacer()
                }
                .padding(.top, 12)
            }
        }
        .padding()
        .background(Color(.controlBackgroundColor))
        .cornerRadius(10)
    }

    // MARK: - Actions

    private func startIdeation() {
        guard !idea.isEmpty, !bridge.ideationRunning else { return }
        errorMessage = nil
        let _ = bridge.startIdeation(idea: idea)
    }
}
