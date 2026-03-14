import SwiftUI

// Ref: FT-SSF-015
// Language picker component for Settings.
// Shows all 40 supported languages with native names and RTL indicator.

struct LanguagePickerView: View {
    @ObservedObject private var l10n = L10n.shared
    @State private var searchText = ""

    private var filteredLanguages: [String] {
        let langs = L10n.supportedLanguages
        guard !searchText.isEmpty else { return langs }
        let query = searchText.lowercased()
        return langs.filter { code in
            code.contains(query) ||
            (L10n.languageNames[code]?.lowercased().contains(query) ?? false)
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: SF.Spacing.sm) {
            // Section header
            HStack {
                Image(systemName: "globe")
                    .foregroundColor(SF.Colors.purple)
                    .font(.system(size: 12))
                Text(l10n.t(.languagePickerTitle))
                    .font(SF.Font.headline)
                    .foregroundColor(SF.Colors.textPrimary)
            }

            // Search field
            HStack(spacing: 6) {
                Image(systemName: "magnifyingglass")
                    .font(.system(size: 11))
                    .foregroundColor(SF.Colors.textMuted)
                TextField(l10n.t(.languagePickerSearch), text: $searchText)
                    .textFieldStyle(.plain)
                    .font(SF.Font.body)
                if !searchText.isEmpty {
                    Button(action: { searchText = "" }) {
                        Image(systemName: "xmark.circle.fill")
                            .font(.system(size: 11))
                            .foregroundColor(SF.Colors.textMuted)
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 5)
            .background(SF.Colors.bgTertiary)
            .clipShape(RoundedRectangle(cornerRadius: SF.Radius.sm))

            // Language grid
            ScrollView {
                LazyVGrid(columns: [
                    GridItem(.flexible(), spacing: 6),
                    GridItem(.flexible(), spacing: 6)
                ], spacing: 4) {
                    ForEach(filteredLanguages, id: \.self) { code in
                        languageRow(code: code)
                    }
                }
            }
            .frame(maxHeight: 300)
        }
        .padding(SF.Spacing.md)
        .background(SF.Colors.bgCard)
        .clipShape(RoundedRectangle(cornerRadius: SF.Radius.md))
    }

    private func languageRow(code: String) -> some View {
        let isSelected = l10n.currentLocale == code
        let isRTL = L10n.rtlLanguages.contains(code)
        let name = L10n.languageNames[code] ?? code

        return Button(action: {
            l10n.setLocale(code)
        }) {
            HStack(spacing: 6) {
                if isRTL {
                    Text("RTL")
                        .font(.system(size: 8, weight: .bold, design: .monospaced))
                        .foregroundColor(SF.Colors.info)
                        .padding(.horizontal, 3)
                        .padding(.vertical, 1)
                        .background(SF.Colors.info.opacity(0.15))
                        .clipShape(RoundedRectangle(cornerRadius: 3))
                }

                Text(name)
                    .font(.system(size: 11, weight: isSelected ? .semibold : .regular))
                    .foregroundColor(isSelected ? SF.Colors.purple : SF.Colors.textPrimary)
                    .lineLimit(1)

                Spacer()

                Text(code.uppercased())
                    .font(.system(size: 9, weight: .medium, design: .monospaced))
                    .foregroundColor(SF.Colors.textMuted)

                if isSelected {
                    Image(systemName: "checkmark.circle.fill")
                        .font(.system(size: 11))
                        .foregroundColor(SF.Colors.purple)
                }
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 5)
            .background(isSelected ? SF.Colors.purple.opacity(0.1) : Color.clear)
            .clipShape(RoundedRectangle(cornerRadius: SF.Radius.sm))
        }
        .buttonStyle(.plain)
    }
}
