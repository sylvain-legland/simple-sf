import SwiftUI

// MARK: - Markdown rendering for agent messages

struct MarkdownView: View {
    let text: String
    let fontSize: CGFloat

    init(_ text: String, fontSize: CGFloat = 14) {
        self.text = text
        self.fontSize = fontSize
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            let blocks = parseBlocks(text)
            ForEach(Array(blocks.enumerated()), id: \.offset) { _, block in
                renderBlock(block)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    // MARK: - Block types

    private enum Block {
        case heading(level: Int, text: String)
        case paragraph(text: String)
        case bullet(text: String, indent: Int)
        case numbered(number: String, text: String)
        case blockquote(text: String)
        case codeBlock(code: String, lang: String)
        case divider
    }

    // MARK: - Parser

    private func parseBlocks(_ raw: String) -> [Block] {
        var blocks: [Block] = []
        let lines = raw.components(separatedBy: "\n")
        var i = 0

        while i < lines.count {
            let line = lines[i]
            let trimmed = line.trimmingCharacters(in: .whitespaces)

            // Empty line
            if trimmed.isEmpty {
                i += 1
                continue
            }

            // Divider
            if trimmed == "---" || trimmed == "***" || trimmed == "___" {
                blocks.append(.divider)
                i += 1
                continue
            }

            // Code block
            if trimmed.hasPrefix("```") {
                let lang = String(trimmed.dropFirst(3)).trimmingCharacters(in: .whitespaces)
                var codeLines: [String] = []
                i += 1
                while i < lines.count {
                    if lines[i].trimmingCharacters(in: .whitespaces).hasPrefix("```") {
                        i += 1
                        break
                    }
                    codeLines.append(lines[i])
                    i += 1
                }
                blocks.append(.codeBlock(code: codeLines.joined(separator: "\n"), lang: lang))
                continue
            }

            // Heading
            if trimmed.hasPrefix("#") {
                let level = trimmed.prefix(while: { $0 == "#" }).count
                if level <= 4 {
                    let headingText = String(trimmed.dropFirst(level)).trimmingCharacters(in: .whitespaces)
                    blocks.append(.heading(level: level, text: headingText))
                    i += 1
                    continue
                }
            }

            // Blockquote
            if trimmed.hasPrefix(">") {
                var quoteLines: [String] = []
                while i < lines.count {
                    let l = lines[i].trimmingCharacters(in: .whitespaces)
                    if l.hasPrefix(">") {
                        quoteLines.append(String(l.dropFirst()).trimmingCharacters(in: .whitespaces))
                    } else if l.isEmpty {
                        break
                    } else {
                        break
                    }
                    i += 1
                }
                blocks.append(.blockquote(text: quoteLines.joined(separator: "\n")))
                continue
            }

            // Bullet list
            if trimmed.hasPrefix("- ") || trimmed.hasPrefix("* ") || trimmed.hasPrefix("+ ") {
                let indent = line.prefix(while: { $0 == " " }).count / 2
                let bulletText = String(trimmed.dropFirst(2))
                blocks.append(.bullet(text: bulletText, indent: indent))
                i += 1
                continue
            }

            // Numbered list
            if let dotIndex = trimmed.firstIndex(of: "."),
               trimmed.distance(from: trimmed.startIndex, to: dotIndex) <= 3,
               trimmed[trimmed.startIndex..<dotIndex].allSatisfy({ $0.isNumber }) {
                let num = String(trimmed[trimmed.startIndex..<dotIndex])
                let rest = String(trimmed[trimmed.index(after: dotIndex)...]).trimmingCharacters(in: .whitespaces)
                blocks.append(.numbered(number: num, text: rest))
                i += 1
                continue
            }

            // Paragraph — gather consecutive non-special lines
            var paraLines: [String] = [trimmed]
            i += 1
            while i < lines.count {
                let nextTrimmed = lines[i].trimmingCharacters(in: .whitespaces)
                if nextTrimmed.isEmpty || nextTrimmed.hasPrefix("#") || nextTrimmed.hasPrefix("```")
                    || nextTrimmed.hasPrefix(">") || nextTrimmed.hasPrefix("- ") || nextTrimmed.hasPrefix("* ")
                    || nextTrimmed == "---" || nextTrimmed == "***" {
                    break
                }
                paraLines.append(nextTrimmed)
                i += 1
            }
            blocks.append(.paragraph(text: paraLines.joined(separator: " ")))
        }

        return blocks
    }

    // MARK: - Renderers

    @ViewBuilder
    private func renderBlock(_ block: Block) -> some View {
        switch block {
        case .heading(let level, let text):
            renderInline(text)
                .font(.system(size: headingSize(level), weight: .bold))
                .foregroundColor(SF.Colors.textPrimary)
                .padding(.top, level == 1 ? 8 : 4)
                .padding(.bottom, 2)

        case .paragraph(let text):
            renderInline(text)
                .font(.system(size: fontSize))
                .foregroundColor(SF.Colors.textPrimary)
                .lineSpacing(5)

        case .bullet(let text, let indent):
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Circle()
                    .fill(SF.Colors.purple.opacity(0.6))
                    .frame(width: 5, height: 5)
                    .offset(y: 1)
                renderInline(text)
                    .font(.system(size: fontSize))
                    .foregroundColor(SF.Colors.textPrimary)
                    .lineSpacing(4)
            }
            .padding(.leading, CGFloat(indent) * 16)

        case .numbered(let number, let text):
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text("\(number).")
                    .font(.system(size: fontSize, weight: .semibold))
                    .foregroundColor(SF.Colors.purple.opacity(0.8))
                    .frame(width: 20, alignment: .trailing)
                renderInline(text)
                    .font(.system(size: fontSize))
                    .foregroundColor(SF.Colors.textPrimary)
                    .lineSpacing(4)
            }

        case .blockquote(let text):
            HStack(spacing: 0) {
                RoundedRectangle(cornerRadius: 1)
                    .fill(SF.Colors.purple.opacity(0.5))
                    .frame(width: 3)
                renderInline(text)
                    .font(.system(size: fontSize))
                    .foregroundColor(SF.Colors.textSecondary)
                    .lineSpacing(4)
                    .padding(.leading, 12)
                    .padding(.vertical, 4)
            }
            .padding(.vertical, 2)

        case .codeBlock(let code, _):
            ScrollView(.horizontal, showsIndicators: false) {
                Text(code)
                    .font(.system(size: fontSize - 1.5).monospaced())
                    .foregroundColor(SF.Colors.textPrimary)
                    .textSelection(.enabled)
            }
            .padding(14)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(SF.Colors.bgPrimary.opacity(0.7))
            .cornerRadius(8)
            .overlay(
                RoundedRectangle(cornerRadius: 8)
                    .stroke(SF.Colors.border, lineWidth: 0.5)
            )

        case .divider:
            Divider()
                .background(SF.Colors.border)
                .padding(.vertical, 4)
        }
    }

    private func headingSize(_ level: Int) -> CGFloat {
        switch level {
        case 1: return fontSize + 6
        case 2: return fontSize + 4
        case 3: return fontSize + 2
        default: return fontSize + 1
        }
    }

    // MARK: - Inline markdown → AttributedString

    private func renderInline(_ text: String) -> Text {
        // SwiftUI Text interprets markdown when initialized with LocalizedStringKey
        // But AttributedString(markdown:) gives better control
        if let attributed = try? AttributedString(markdown: text, options: .init(interpretedSyntax: .inlineOnlyPreservingWhitespace)) {
            return Text(attributed)
        }
        return Text(text)
    }
}
