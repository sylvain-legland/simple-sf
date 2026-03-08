import SwiftUI

struct IdeationAgent: Identifiable {
    let id = UUID()
    let name: String
    let role: String
    let icon: String
    let systemPrompt: String
    var response: String = ""
    var isLoading = false
}

@MainActor
struct IdeationView: View {
    @ObservedObject private var llm = LLMService.shared

    @State private var idea = ""
    @State private var agents: [IdeationAgent] = Self.defaultAgents
    @State private var isRunning = false
    @State private var errorMessage: String?
    @State private var showSettings = false

    private static var defaultAgents: [IdeationAgent] = [
        IdeationAgent(name: "Product Manager", role: "pm", icon: "chart.bar.fill",
            systemPrompt: "You are an experienced Product Manager. Analyze the given idea from a product perspective: user value, market fit, MVP scope, key risks. Be concise (150 words max)."),
        IdeationAgent(name: "Tech Lead", role: "tech", icon: "chevron.left.forwardslash.chevron.right",
            systemPrompt: "You are a senior Tech Lead. Analyze the given idea technically: recommended stack, architecture, key technical challenges, estimated complexity. Be concise (150 words max)."),
        IdeationAgent(name: "UX Designer", role: "ux", icon: "paintbrush.fill",
            systemPrompt: "You are a UX/Product Designer. Analyze the given idea from a UX perspective: core user flows, key screens, accessibility, main UX pitfalls. Be concise (150 words max).")
    ]

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Image(systemName: "lightbulb.fill")
                    .foregroundColor(.yellow)
                Text("Ideation")
                    .font(.title2.bold())
                Spacer()
                if isRunning {
                    ProgressView().scaleEffect(0.8)
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
                            Button("Analyze with 3 agents") {
                                Task { await runIdeation() }
                            }
                            .buttonStyle(.borderedProminent)
                            .disabled(idea.isEmpty || isRunning || llm.activeProvider == nil)

                            if llm.activeProvider == nil {
                                Label("No provider configured", systemImage: "exclamationmark.triangle")
                                    .font(.caption)
                                    .foregroundColor(.orange)
                            }
                        }
                    }
                    .padding()
                    .background(Color(.controlBackgroundColor))
                    .cornerRadius(10)

                    // Agent responses
                    ForEach($agents) { $agent in
                        AgentResponseCard(agent: $agent)
                    }

                    // Synthesis (shown when all done)
                    if agents.allSatisfy({ !$0.response.isEmpty }) {
                        synthesisCard
                    }
                }
                .padding()
            }
        }
    }

    private var synthesisCard: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Image(systemName: "star.fill").foregroundColor(.purple)
                Text("Three perspectives gathered").font(.headline)
                Spacer()
                Button("Reset") {
                    agents = Self.defaultAgents
                    idea = ""
                }
                .buttonStyle(.plain)
                .foregroundColor(.secondary)
            }
            Text("You now have product, technical, and UX perspectives on your idea. Use the chat (Jarvis) to go deeper on any aspect.")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding()
        .background(Color.purple.opacity(0.1))
        .cornerRadius(10)
    }

    private func runIdeation() async {
        guard !idea.isEmpty, llm.activeProvider != nil else { return }
        isRunning = true
        errorMessage = nil
        // Reset
        for i in agents.indices { agents[i].response = ""; agents[i].isLoading = false }

        let ideaText = idea
        // Run agents sequentially (each one sees the previous response for context)
        var context = "Idea: \(ideaText)\n\n"

        for i in agents.indices {
            agents[i].isLoading = true
            do {
                let prompt = context.isEmpty ? ideaText : "\(ideaText)\n\n---\nContext from previous analysis:\n\(context)"
                let resp = try await llm.complete(
                    messages: [LLMMessage(role: "user", content: prompt)],
                    system: agents[i].systemPrompt
                )
                agents[i].response = resp
                agents[i].isLoading = false
                context += "\(agents[i].role.uppercased()) perspective: \(resp)\n\n"
            } catch {
                agents[i].response = "Error: \(error.localizedDescription)"
                agents[i].isLoading = false
                errorMessage = error.localizedDescription
                break
            }
        }
        isRunning = false
    }
}

struct AgentResponseCard: View {
    @Binding var agent: IdeationAgent

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Image(systemName: agent.icon).foregroundColor(.purple)
                Text(agent.name).font(.headline)
                Spacer()
                if agent.isLoading { ProgressView().scaleEffect(0.7) }
                else if !agent.response.isEmpty {
                    Image(systemName: "checkmark.circle.fill").foregroundColor(.green).font(.caption)
                }
            }
            if agent.isLoading {
                HStack(spacing: 4) {
                    ForEach(0..<3) { i in
                        Circle().fill(Color.purple.opacity(0.6)).frame(width: 6, height: 6)
                            .animation(.easeInOut(duration: 0.6).repeatForever().delay(Double(i) * 0.2), value: agent.isLoading)
                    }
                }
                .padding(.vertical, 8)
            } else if !agent.response.isEmpty {
                Text(agent.response)
                    .font(.body)
                    .textSelection(.enabled)
            } else {
                Text("Waiting…")
                    .foregroundColor(.secondary)
                    .font(.caption)
            }
        }
        .padding()
        .background(Color(.controlBackgroundColor))
        .cornerRadius(10)
    }
}
