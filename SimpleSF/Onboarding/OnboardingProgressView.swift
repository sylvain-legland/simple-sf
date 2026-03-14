import SwiftUI

// Ref: FT-SSF-007
// Expandable cloud provider card with API key management and test status.

struct CloudProviderCard: View {
    let provider: LLMProvider
    let storedKey: String?
    @Binding var draftKey: String
    @Binding var modelOverride: String
    let isSelected: Bool
    let isTesting: Bool
    let testResult: Bool?
    let onSave: () -> Void
    let onTest: () -> Void
    let onDelete: () -> Void
    let onUse: () -> Void

    @State private var expanded = false

    private var hasKey: Bool { storedKey != nil && !storedKey!.isEmpty }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Row header
            Button(action: { withAnimation(.spring(response: 0.3)) { expanded.toggle() } }) {
                HStack(spacing: 10) {
                    if isSelected {
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundColor(.green)
                            .font(.system(size: 14))
                    } else {
                        Circle()
                            .fill(hasKey ? Color.green : SF.Colors.textMuted.opacity(0.3))
                            .frame(width: 8, height: 8)
                    }

                    Text(provider.displayName)
                        .font(.headline)
                        .foregroundColor(SF.Colors.textPrimary)

                    Text(provider.subtitle)
                        .font(.caption2)
                        .foregroundColor(SF.Colors.textMuted)

                    Spacer()

                    if hasKey {
                        if isTesting {
                            ProgressView().scaleEffect(0.7)
                        } else if let ok = testResult {
                            Image(systemName: ok ? "checkmark.circle.fill" : "xmark.circle.fill")
                                .foregroundColor(ok ? .green : .red)
                        }

                        if !isSelected {
                            Button(action: onUse) {
                                Text("Use")
                                    .font(.caption.bold())
                            }
                            .buttonStyle(.borderedProminent)
                            .tint(SF.Colors.purple)
                            .controlSize(.mini)
                        } else {
                            Label("Active", systemImage: "checkmark.circle.fill")
                                .font(.caption.bold())
                                .foregroundColor(.green)
                        }
                    }

                    Image(systemName: expanded ? "chevron.up" : "chevron.down")
                        .font(.caption)
                        .foregroundColor(SF.Colors.textMuted)
                }
                .padding(12)
            }
            .buttonStyle(.plain)

            if expanded {
                Divider().background(SF.Colors.border)
                VStack(alignment: .leading, spacing: 10) {
                    // Model override
                    HStack {
                        Text("Model:")
                            .font(.callout)
                            .foregroundColor(SF.Colors.textSecondary)
                        TextField(provider.defaultModel, text: $modelOverride)
                            .textFieldStyle(.roundedBorder)
                            .frame(maxWidth: 250)
                            .onChange(of: modelOverride) { newValue in
                                UserDefaults.standard.set(newValue, forKey: "sf_model_\(provider.rawValue)")
                                if isSelected {
                                    AppState.shared.setActiveProvider(provider, model: newValue.isEmpty ? nil : newValue)
                                }
                            }
                    }

                    // API key
                    HStack {
                        SecureField("API Key", text: $draftKey)
                            .textFieldStyle(.roundedBorder)
                        Button("Save") { onSave() }
                            .buttonStyle(.bordered)
                        if hasKey {
                            Button("Test") { onTest() }
                                .buttonStyle(.bordered)
                                .tint(.green)
                            Button("Delete", role: .destructive) { onDelete(); draftKey = "" }
                                .buttonStyle(.plain)
                                .foregroundColor(.red)
                        }
                    }
                    Button("Get API key →") {
                        NSWorkspace.shared.open(URL(string: provider.docURL)!)
                    }
                    .font(.caption)
                    .buttonStyle(.plain)
                    .foregroundColor(SF.Colors.purple)
                }
                .padding(12)
            }
        }
        .background(isSelected ? SF.Colors.purple.opacity(0.08) : SF.Colors.bgTertiary.opacity(0.5))
        .cornerRadius(10)
        .overlay(
            RoundedRectangle(cornerRadius: 10)
                .stroke(isSelected ? SF.Colors.purple.opacity(0.5) : Color.clear, lineWidth: 1.5)
        )
    }
}
