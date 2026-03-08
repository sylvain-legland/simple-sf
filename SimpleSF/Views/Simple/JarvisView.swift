import SwiftUI

struct JarvisView: View {
    @State private var messages: [ChatMessage] = []
    @State private var input = ""
    @State private var isStreaming = false
    @State private var sessionId: String?
    @State private var streamTask: Task<Void, Never>?

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Image(systemName: "bubble.left.and.bubble.right.fill")
                    .foregroundStyle(.purple)
                Text("Jarvis").font(.title2.bold())
                Spacer()
                Button { clearChat() } label: {
                    Label("Clear", systemImage: "trash")
                }.disabled(messages.isEmpty)
            }
            .padding(.horizontal, 24).padding(.vertical, 16)

            Divider()

            // Messages
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 12) {
                        if messages.isEmpty {
                            JarvisWelcome()
                        }
                        ForEach(messages) { msg in
                            ChatBubble(message: msg)
                                .id(msg.id)
                        }
                    }
                    .padding(20)
                }
                .onChange(of: messages.count) { _, _ in
                    if let last = messages.last {
                        withAnimation { proxy.scrollTo(last.id, anchor: .bottom) }
                    }
                }
            }

            Divider()

            // Input bar
            HStack(spacing: 12) {
                TextField("Ask Jarvis anything…", text: $input, axis: .vertical)
                    .textFieldStyle(.roundedBorder)
                    .lineLimit(1...6)
                    .onSubmit { if !isStreaming { Task { await send() } } }

                if isStreaming {
                    Button { streamTask?.cancel(); isStreaming = false } label: {
                        Image(systemName: "stop.circle.fill")
                            .resizable().scaledToFit().frame(width: 28)
                            .foregroundStyle(.red)
                    }.buttonStyle(.plain)
                } else {
                    Button { Task { await send() } } label: {
                        Image(systemName: "arrow.up.circle.fill")
                            .resizable().scaledToFit().frame(width: 28)
                            .foregroundStyle(.purple)
                    }
                    .buttonStyle(.plain)
                    .disabled(input.trimmingCharacters(in: .whitespaces).isEmpty)
                    .keyboardShortcut(.return, modifiers: .command)
                }
            }
            .padding(.horizontal, 20).padding(.vertical, 14)
        }
        .task { await initSession() }
    }

    private func initSession() async {
        do {
            let result = try await SFClient.shared.postRaw("/api/sessions/quick", body: [
                "agent_id": "jarvis",
                "mode": "chat"
            ]) as! [String: Any]
            sessionId = result["session_id"] as? String
        } catch {}
    }

    private func send() async {
        let text = input.trimmingCharacters(in: .whitespaces)
        guard !text.isEmpty else { return }
        input = ""
        let userMsg = ChatMessage(role: .user, content: text)
        messages.append(userMsg)

        let assistantMsg = ChatMessage(role: .assistant, content: "")
        messages.append(assistantMsg)
        isStreaming = true

        streamTask = Task {
            do {
                let path = "/api/sessions/\(sessionId ?? "default")/chat"
                for try await chunk in SFClient.shared.stream(path, body: ["message": text]) {
                    if Task.isCancelled { break }
                    if let data = chunk.data(using: .utf8),
                       let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                       let delta = json["delta"] as? String ?? json["content"] as? String {
                        let idx = messages.indices.last!
                        messages[idx].content += delta
                    }
                }
            } catch {}
            isStreaming = false
        }
    }

    private func clearChat() {
        messages = []
        sessionId = nil
        Task { await initSession() }
    }
}

struct ChatMessage: Identifiable {
    let id = UUID()
    var role: Role
    var content: String
    enum Role { case user, assistant }
}

struct ChatBubble: View {
    let message: ChatMessage

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            if message.role == .user { Spacer(minLength: 60) }

            if message.role == .assistant {
                Image(systemName: "cpu.fill")
                    .foregroundStyle(.purple)
                    .frame(width: 28, height: 28)
                    .background(Circle().fill(.purple.opacity(0.15)))
            }

            Text(message.content.isEmpty ? "…" : message.content)
                .textSelection(.enabled)
                .padding(.horizontal, 14).padding(.vertical, 10)
                .background(
                    RoundedRectangle(cornerRadius: 14)
                        .fill(message.role == .user ? Color.purple : Color.secondary.opacity(0.12))
                )
                .foregroundStyle(message.role == .user ? .white : .primary)

            if message.role == .user {
                Image(systemName: "person.crop.circle.fill")
                    .foregroundStyle(.secondary)
                    .frame(width: 28)
            } else {
                Spacer(minLength: 60)
            }
        }
    }
}

struct JarvisWelcome: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "cpu.fill")
                .resizable().scaledToFit().frame(width: 48)
                .foregroundStyle(.purple)
            Text("Hi, I'm Jarvis").font(.title2.bold())
            Text("Your AI assistant. Ask me anything about your projects, agents, or the platform.")
                .multilineTextAlignment(.center)
                .foregroundStyle(.secondary)
                .frame(maxWidth: 360)
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 80)
    }
}
