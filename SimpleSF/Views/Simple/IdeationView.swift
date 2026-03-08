import SwiftUI

struct IdeationView: View {
    @State private var topic = ""
    @State private var session: IdeationSession?
    @State private var isRunning = false
    @State private var error: String?

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Image(systemName: "lightbulb.fill").foregroundStyle(.yellow)
                Text("Ideation").font(.title2.bold())
                Spacer()
                if let session, !isRunning {
                    Button("New session") { self.session = nil; topic = "" }
                }
            }
            .padding(.horizontal, 24).padding(.vertical, 16)

            Divider()

            if session == nil {
                IdeationTopicInput(topic: $topic, isRunning: isRunning) {
                    await startSession()
                }
            } else {
                IdeationSessionView(session: session!, isRunning: isRunning)
            }
        }
    }

    private func startSession() async {
        guard !topic.isEmpty else { return }
        isRunning = true
        error = nil
        do {
            let result = try await SFClient.shared.postRaw("/api/ideation/sessions", body: [
                "topic": topic
            ]) as! [String: Any]
            let sid = result["session_id"] as? String ?? ""
            session = IdeationSession(id: sid, topic: topic, messages: [])
            // Poll for messages
            await pollSession(id: sid)
        } catch {
            self.error = error.localizedDescription
        }
        isRunning = false
    }

    private func pollSession(id: String) async {
        for _ in 0..<60 {
            try? await Task.sleep(nanoseconds: 2_000_000_000)
            do {
                let result = try await SFClient.shared.getRaw("/api/ideation/sessions/\(id)") as! [String: Any]
                let msgs = (result["messages"] as? [[String: Any]]) ?? []
                let parsed = msgs.compactMap { d -> IdeationMessage? in
                    guard let agent = d["agent_name"] as? String,
                          let content = d["content"] as? String else { return nil }
                    return IdeationMessage(agentName: agent, content: content,
                                           role: d["role"] as? String ?? "assistant")
                }
                session?.messages = parsed
                let status = result["status"] as? String ?? ""
                if status == "completed" || status == "failed" { break }
            } catch { break }
        }
    }
}

struct IdeationTopicInput: View {
    @Binding var topic: String
    let isRunning: Bool
    let onStart: () async -> Void

    var body: some View {
        VStack(spacing: 24) {
            Spacer()
            Image(systemName: "person.3.fill")
                .resizable().scaledToFit().frame(width: 56)
                .foregroundStyle(.purple)
            Text("Start an ideation session").font(.title2.bold())
            Text("A team of AI agents will brainstorm, debate, and build on each other's ideas.")
                .multilineTextAlignment(.center).foregroundStyle(.secondary)
                .frame(maxWidth: 420)
            HStack {
                TextField("Topic or question…", text: $topic, axis: .vertical)
                    .textFieldStyle(.roundedBorder).frame(maxWidth: 440)
                    .lineLimit(3...6)
                Button {
                    Task { await onStart() }
                } label: {
                    if isRunning {
                        ProgressView().scaleEffect(0.8).frame(width: 60)
                    } else {
                        Text("Start")
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(topic.isEmpty || isRunning)
            }
            Spacer()
        }
        .padding(40)
    }
}

struct IdeationSessionView: View {
    let session: IdeationSession
    let isRunning: Bool

    var body: some View {
        VStack(spacing: 0) {
            // Topic bar
            HStack {
                Image(systemName: "lightbulb").foregroundStyle(.yellow)
                Text(session.topic).font(.headline)
                Spacer()
                if isRunning {
                    HStack(spacing: 6) {
                        ProgressView().scaleEffect(0.8)
                        Text("Team is thinking…").font(.caption).foregroundStyle(.secondary)
                    }
                } else {
                    Label("Complete", systemImage: "checkmark.circle.fill")
                        .font(.caption).foregroundStyle(.green)
                }
            }
            .padding(.horizontal, 24).padding(.vertical, 12)
            .background(.secondary.opacity(0.06))

            Divider()

            ScrollView {
                LazyVStack(alignment: .leading, spacing: 16) {
                    ForEach(session.messages) { msg in
                        IdeationBubble(message: msg)
                    }
                    if isRunning {
                        HStack(spacing: 8) {
                            ProgressView().scaleEffect(0.7)
                            Text("Waiting for next contribution…")
                                .font(.caption).foregroundStyle(.tertiary)
                        }
                        .padding(.leading, 16)
                    }
                }
                .padding(24)
            }
        }
    }
}

struct IdeationBubble: View {
    let message: IdeationMessage

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            // Agent avatar
            ZStack {
                Circle().fill(agentColor.opacity(0.2)).frame(width: 36, height: 36)
                Text(String(message.agentName.prefix(1)).uppercased())
                    .font(.headline).foregroundStyle(agentColor)
            }

            VStack(alignment: .leading, spacing: 6) {
                Text(message.agentName)
                    .font(.caption.bold()).foregroundStyle(agentColor)
                Text(message.content)
                    .font(.body).textSelection(.enabled)
                    .padding(12)
                    .background(RoundedRectangle(cornerRadius: 10).fill(.secondary.opacity(0.08)))
            }
        }
    }

    private var agentColor: Color {
        let colors: [Color] = [.purple, .blue, .green, .orange, .pink, .teal, .indigo]
        let idx = abs(message.agentName.hashValue) % colors.count
        return colors[idx]
    }
}

// MARK: - Models

struct IdeationSession {
    let id: String
    let topic: String
    var messages: [IdeationMessage]
}

struct IdeationMessage: Identifiable {
    let id = UUID()
    let agentName: String
    let content: String
    let role: String
}
