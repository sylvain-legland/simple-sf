import SwiftUI

// Ref: FT-SSF-010
struct AgentsView: View {
    @ObservedObject private var bridge = SFBridge.shared
    @State private var agents: [SFBridge.SFAgent] = []
    @State private var loadingState: LoadingState = .loading  // Ref: FT-SSF-013

    var body: some View {
        VStack(spacing: 0) {
            IHMContextHeader(context: .agents)

            HStack {
                Image(systemName: "person.3.fill")
                    .font(.title2)
                    .foregroundColor(.purple)
                Text("Agent Team")
                    .font(.title2.bold())
                Spacer()
                if loadingState == .loaded {
                    Text("\(agents.count) agents")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .padding()

            Divider()

            // Ref: FT-SSF-013 — Skeleton loading
            LoadingStateView(
                state: loadingState,
                skeleton: { SkeletonAgentGrid(count: 6) },
                content: {
                    ScrollView {
                        LazyVGrid(columns: [GridItem(.adaptive(minimum: 250), spacing: 16)], spacing: 16) {
                            ForEach(agents) { agent in
                                agentCard(agent)
                            }
                        }
                        .padding()
                    }
                },
                onRetry: { loadAgents() },
                emptyAction: { loadAgents() },
                emptyActionLabel: "Reload"
            )
        }
        .onAppear { loadAgents() }
    }

    private func loadAgents() {
        loadingState = .loading
        let result = bridge.listAgents()
        agents = result
        loadingState = result.isEmpty ? .empty("No agents available yet.") : .loaded
    }

    private func agentCard(_ agent: SFBridge.SFAgent) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 10) {
                Text(String(agent.name.prefix(1)))
                    .font(.title3.bold())
                    .frame(width: 36, height: 36)
                    .background(roleColor(agent.role))
                    .foregroundColor(.white)
                    .clipShape(Circle())

                VStack(alignment: .leading) {
                    Text(agent.name)
                        .font(.headline)
                    Text(agent.role)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(roleColor(agent.role).opacity(0.15))
                        .cornerRadius(4)
                }
            }

            Text(agent.persona)
                .font(.caption)
                .foregroundColor(.secondary)
                .lineLimit(3)
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.gray.opacity(0.05))
        .cornerRadius(12)
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(roleColor(agent.role).opacity(0.2), lineWidth: 1)
        )
    }

    private func roleColor(_ role: String) -> Color {
        switch role {
        case "RTE":      return .blue
        case "PO":       return .green
        case "Lead Dev": return .orange
        case "Frontend": return .purple
        case "Backend":  return .indigo
        case "QA":       return .red
        default:         return .gray
        }
    }
}
