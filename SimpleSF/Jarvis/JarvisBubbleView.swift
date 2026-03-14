import SwiftUI

// Ref: FT-SSF-001
// Message bubble for user/assistant messages in the Jarvis chat.

struct MessageBubble: View {
    let message: LLMMessage

    var body: some View {
        let isUser = message.role == "user"
        HStack(alignment: .top, spacing: 12) {
            if isUser {
                Spacer(minLength: 80)
            }

            if !isUser {
                Image(systemName: "sparkles")
                    .font(.system(size: 14))
                    .foregroundColor(SF.Colors.purple)
                    .frame(width: 36, height: 36)
                    .background(SF.Colors.purple.opacity(0.12))
                    .clipShape(Circle())
            }

            if isUser {
                Text(message.content)
                    .font(.system(size: 14))
                    .foregroundColor(SF.Colors.textPrimary)
                    .lineSpacing(4)
                    .textSelection(.enabled)
                    .padding(.horizontal, 16)
                    .padding(.vertical, 12)
                    .background(SF.Colors.purple.opacity(0.12))
                    .cornerRadius(16)
                    .overlay(
                        RoundedRectangle(cornerRadius: 16)
                            .stroke(SF.Colors.purple.opacity(0.2), lineWidth: 0.5)
                    )
                    .frame(maxWidth: 700, alignment: .trailing)
            } else {
                MarkdownView(message.content, fontSize: 14)
                    .textSelection(.enabled)
                    .padding(.horizontal, 16)
                    .padding(.vertical, 12)
                    .background(SF.Colors.bgSecondary)
                    .cornerRadius(12)
                    .overlay(
                        RoundedRectangle(cornerRadius: 12)
                            .stroke(SF.Colors.border, lineWidth: 0.5)
                    )
                    .frame(maxWidth: 700, alignment: .leading)
            }

            if isUser {
                Image(systemName: "person.circle.fill")
                    .font(.system(size: 16))
                    .foregroundColor(SF.Colors.textSecondary)
                    .frame(width: 36, height: 36)
            }

            if !isUser {
                Spacer(minLength: 80)
            }
        }
    }
}
