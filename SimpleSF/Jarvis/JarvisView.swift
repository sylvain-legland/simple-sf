import SwiftUI

@MainActor
struct JarvisView: View {
    @ObservedObject private var llm = LLMService.shared
    @ObservedObject private var chatStore = ChatStore.shared
    @ObservedObject private var keychain = KeychainService.shared

    @State private var inputText = ""
    @State private var isStreaming = false
    @State private var streamingContent = ""
    @State private var errorMessage: String?

    private var session: ChatSession? { chatStore.activeSession }
    private var messages: [LLMMessage] { session?.messages ?? [] }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Image(systemName: "sparkles")
                    .foregroundColor(.purple)
                Text("Jarvis")
                    .font(.title2.bold())
                Spacer()
                if let prov = llm.activeProvider {
                    Label(prov.displayName, systemImage: "checkmark.circle.fill")
                        .font(.caption)
                        .foregroundColor(.green)
                } else {
                    Label("No provider", systemImage: "exclamationmark.circle")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
                Button(action: newChat) {
                    Image(systemName: "square.and.pencil")
                }
                .buttonStyle(.plain)
            }
            .padding()

            Divider()

            // Messages
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 12) {
                        if messages.isEmpty {
                            emptyState
                        }
                        ForEach(Array(messages.enumerated()), id: \.offset) { _, msg in
                            MessageRow(message: msg)
                        }
                        if isStreaming {
                            MessageRow(message: LLMMessage(role: "assistant", content: streamingContent + "▊"))
                        }
                        if let err = errorMessage {
                            Text(err)
                                .foregroundColor(.red)
                                .font(.caption)
                                .padding(.horizontal)
                        }
                    }
                    .padding()
                    .id("bottom")
                }
                .onChange(of: streamingContent) { _ in
                    withAnimation { proxy.scrollTo("bottom", anchor: .bottom) }
                }
                .onChange(of: messages.count) { _ in
                    withAnimation { proxy.scrollTo("bottom", anchor: .bottom) }
                }
            }

            Divider()

            // Input
            HStack(spacing: 8) {
                TextField("Message Jarvis…", text: $inputText, axis: .vertical)
                    .lineLimit(1...6)
                    .textFieldStyle(.plain)
                    .padding(10)
                    .background(Color(.controlBackgroundColor))
                    .cornerRadius(8)
                    .onSubmit { if !inputText.isEmpty { Task { await sendMessage() } } }

                Button(action: { Task { await sendMessage() } }) {
                    Image(systemName: isStreaming ? "stop.circle.fill" : "arrow.up.circle.fill")
                        .font(.system(size: 28))
                        .foregroundColor(isStreaming ? .red : .purple)
                }
                .buttonStyle(.plain)
                .disabled(inputText.isEmpty && !isStreaming)
            }
            .padding()
        }
        .onAppear {
            if chatStore.activeSession == nil { chatStore.newSession() }
        }
    }

    private var emptyState: some View {
        VStack(spacing: 16) {
            Image(systemName: "sparkles")
                .font(.system(size: 48))
                .foregroundColor(.purple.opacity(0.4))
            Text("Ask Jarvis anything")
                .font(.title3)
                .foregroundColor(.secondary)
            Text("Your local AI assistant. All API calls go directly\nto the provider you configured in Settings.")
                .font(.caption)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 60)
    }

    private func newChat() {
        chatStore.newSession()
        errorMessage = nil
    }

    private func sendMessage() async {
        guard !inputText.isEmpty, !isStreaming else { return }
        guard llm.activeProvider != nil else {
            errorMessage = "Configure an API key in Settings first."
            return
        }

        let userText = inputText
        inputText = ""
        errorMessage = nil

        // Persist user msg
        let sid = session?.id ?? chatStore.newSession().id
        chatStore.appendMessage(LLMMessage(role: "user", content: userText), to: sid)

        // Stream assistant reply
        isStreaming = true
        streamingContent = ""

        let history = chatStore.sessions.first(where: { $0.id == sid })?.messages ?? []
        let stream = llm.stream(
            messages: history,
            system: "You are Jarvis, a helpful software engineering AI assistant. Be concise and precise."
        )

        for await chunk in stream {
            streamingContent += chunk
        }

        // Persist full reply
        if !streamingContent.isEmpty {
            chatStore.appendMessage(LLMMessage(role: "assistant", content: streamingContent), to: sid)
        }
        streamingContent = ""
        isStreaming = false
    }
}

struct MessageRow: View {
    let message: LLMMessage
    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: message.role == "user" ? "person.circle" : "sparkles")
                .foregroundColor(message.role == "user" ? .blue : .purple)
                .frame(width: 20)
            Text(message.content)
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.vertical, 4)
    }
}
