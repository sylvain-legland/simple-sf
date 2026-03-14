import SwiftUI

// Ref: FT-SSF-003

// MARK: - Event Feed Views (extension on ProjectAccordion)

extension ProjectAccordion {

    // MARK: - Display Item Model

    struct DisplayItem: Identifiable {
        let id: String
        enum Kind {
            case message(SFBridge.AgentEvent)
            case toolGroup([SFBridge.AgentEvent])
        }
        let kind: Kind
    }

    // MARK: - Build Display Items

    /// Pre-process events: collapse tool_call/tool_result into compact groups,
    /// filter out thinking events (shown inline), count only substantive messages.
    func buildDisplayItems(_ raw: [SFBridge.AgentEvent]) -> [DisplayItem] {
        var items: [DisplayItem] = []
        var pendingTools: [SFBridge.AgentEvent] = []
        let agentsWithResponse = Set(raw.filter {
            $0.eventType == "response" || $0.eventType == "discuss_response" || $0.eventType == "response_chunk"
        }.map { $0.agentId })

        func flushTools() {
            guard !pendingTools.isEmpty else { return }
            let id = pendingTools.first!.id.uuidString
            items.append(DisplayItem(id: "tg-\(id)", kind: .toolGroup(pendingTools)))
            pendingTools = []
        }

        for event in raw {
            if (event.eventType == "thinking" || event.eventType == "discuss_thinking"),
               agentsWithResponse.contains(event.agentId) {
                continue
            }
            if event.eventType == "tool_call" || event.eventType == "tool_result" {
                pendingTools.append(event)
            } else {
                flushTools()
                items.append(DisplayItem(id: event.id.uuidString, kind: .message(event)))
            }
        }
        flushTools()
        return items
    }

    // MARK: - Event Scroll Feed

    func eventScrollFeed(events feedEvents: [SFBridge.AgentEvent]) -> some View {
        let displayItems = buildDisplayItems(feedEvents)
        let messageCount = feedEvents.filter { $0.eventType == "response" || $0.eventType == "discuss_response" }.count

        return ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 6) {
                    if let pat = currentPhasePattern, !pat.isEmpty {
                        HStack(spacing: 6) {
                            PatternBadge(pattern: pat)
                            Text("·")
                                .foregroundColor(SF.Colors.textMuted)
                            Text("\(messageCount) messages")
                                .font(.system(size: 10))
                                .foregroundColor(SF.Colors.textMuted)
                            Spacer()
                        }
                        .padding(.bottom, 4)
                    }

                    ForEach(displayItems) { item in
                        switch item.kind {
                        case .message(let event):
                            eventRow(event, isLast: item.id == displayItems.last?.id && !(isActive && bridge.isRunning))
                                .id(item.id)
                        case .toolGroup(let tools):
                            toolBadgeRow(tools)
                                .id(item.id)
                        }
                    }

                    if isActive && bridge.isRunning {
                        streamingIndicator.id("streaming-tail")
                    }
                }
                .padding(16)
            }
            .onChange(of: feedEvents.count) { _, _ in
                withAnimation {
                    if isActive && bridge.isRunning {
                        proxy.scrollTo("streaming-tail", anchor: .bottom)
                    } else if let last = displayItems.last {
                        proxy.scrollTo(last.id, anchor: .bottom)
                    }
                }
            }
        }
    }

    // MARK: - Live / Global Feed Accessors

    var liveEventsFeed: some View {
        eventScrollFeed(events: projectEvents)
    }

    var globalEventsFeed: some View {
        eventScrollFeed(events: bridge.events)
    }

    // MARK: - Streaming Indicator

    var streamingIndicator: some View {
        HStack(spacing: 8) {
            ProgressView()
                .controlSize(.small)
                .tint(SF.Colors.purple)
            Text(bridge.isReasoning ? "Raisonnement en profondeur…" : "Agents en cours de réflexion…")
                .font(.system(size: 11, weight: .medium))
                .foregroundColor(bridge.isReasoning ? SF.Colors.purple : SF.Colors.textMuted)
            Spacer()
        }
        .padding(10)
        .background(SF.Colors.purple.opacity(bridge.isReasoning ? 0.12 : 0.06))
        .cornerRadius(6)
    }

    // MARK: - Tool Badge Row

    /// Compact inline badges for tool_call / tool_result groups
    func toolBadgeRow(_ tools: [SFBridge.AgentEvent]) -> some View {
        let agentId = tools.first?.agentId ?? "unknown"
        let agentName = tools.first.flatMap { !$0.agentName.isEmpty ? $0.agentName : nil }
            ?? catalog.agentName(agentId)
        let agentColor = catalog.agentColor(agentId)

        let badges: [(icon: String, label: String)] = tools.compactMap { event in
            guard event.eventType == "tool_call" else { return nil }
            let raw = event.data
            let toolName: String
            if let pipeIdx = raw.firstIndex(of: "|") {
                toolName = String(raw[raw.startIndex..<pipeIdx]).trimmingCharacters(in: .whitespaces)
            } else if let parenIdx = raw.firstIndex(of: "(") {
                toolName = String(raw[raw.startIndex..<parenIdx]).trimmingCharacters(in: .whitespaces)
            } else {
                toolName = raw.trimmingCharacters(in: .whitespaces)
            }
            guard !toolName.isEmpty else { return nil }

            let shortName = toolName
                .replacingOccurrences(of: "code_", with: "")
                .replacingOccurrences(of: "file_", with: "")
                .replacingOccurrences(of: "memory_", with: "mem:")
                .replacingOccurrences(of: "git_", with: "git:")
                .replacingOccurrences(of: "deep_", with: "")
                .replacingOccurrences(of: "list_", with: "ls:")

            let icon: String
            switch true {
            case toolName.contains("read"):   icon = "doc.text"
            case toolName.contains("write"):  icon = "square.and.pencil"
            case toolName.contains("edit"):   icon = "pencil"
            case toolName.contains("search"): icon = "magnifyingglass"
            case toolName.contains("list"):   icon = "list.bullet"
            case toolName.contains("git"):    icon = "arrow.triangle.branch"
            case toolName.contains("memory"): icon = "brain"
            case toolName.contains("build"):  icon = "hammer"
            case toolName.contains("test"):   icon = "checkmark.shield"
            default:                           icon = "wrench"
            }
            return (icon, shortName)
        }

        // Deduplicate consecutive identical badges
        var deduped: [(icon: String, label: String, count: Int)] = []
        for badge in badges {
            if let last = deduped.last, last.icon == badge.icon && last.label == badge.label {
                deduped[deduped.count - 1].count += 1
            } else {
                deduped.append((badge.icon, badge.label, 1))
            }
        }

        guard !deduped.isEmpty else { return AnyView(EmptyView()) }

        return AnyView(
            HStack(spacing: 6) {
                AgentAvatarView(agentId: agentId, size: 20)
                    .overlay(Circle().stroke(agentColor.opacity(0.5), lineWidth: 1))
                Text(agentName)
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundColor(agentColor)
                Image(systemName: "gearshape.2")
                    .font(.system(size: 9))
                    .foregroundColor(SF.Colors.textMuted)
                ForEach(Array(deduped.enumerated()), id: \.offset) { _, badge in
                    HStack(spacing: 3) {
                        Image(systemName: badge.icon)
                            .font(.system(size: 9))
                        Text(badge.count > 1 ? "\(badge.label) ×\(badge.count)" : badge.label)
                            .font(.system(size: 9, weight: .medium, design: .monospaced))
                    }
                    .foregroundColor(SF.Colors.textSecondary)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 3)
                    .background(SF.Colors.bgTertiary)
                    .cornerRadius(4)
                }
                Spacer()
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
        )
    }

    // MARK: - Event Row

    func eventRow(_ event: SFBridge.AgentEvent, isLast: Bool = false) -> some View {
        let color = catalog.agentColor(event.agentId)
        let displayRole = !event.role.isEmpty ? event.role : catalog.agentRole(event.agentId)
        let displayName = !event.agentName.isEmpty ? event.agentName : catalog.agentName(event.agentId)
        let mtype = event.messageType.isEmpty ? "response" : event.messageType
        let borderColor = eventBorderColor(mtype)

        if event.eventType == "thinking" || event.eventType == "discuss_thinking" {
            return AnyView(thinkingRow(event))
        }

        return AnyView(
            HStack(alignment: .top, spacing: 0) {
                RoundedRectangle(cornerRadius: 2)
                    .fill(borderColor)
                    .frame(width: 3)

                VStack(alignment: .leading, spacing: 8) {
                    HStack(spacing: 8) {
                        AgentAvatarView(agentId: event.agentId, size: 32)
                            .overlay(Circle().stroke(color.opacity(0.6), lineWidth: 2))

                        VStack(alignment: .leading, spacing: 2) {
                            HStack(spacing: 6) {
                                Text(displayName)
                                    .font(.system(size: 13, weight: .bold))
                                    .foregroundColor(color)
                                if mtype != "response" {
                                    eventTypeBadge(mtype)
                                }
                                if event.round > 0 {
                                    Text("R\(event.round)")
                                        .font(.system(size: 9, weight: .bold, design: .monospaced))
                                        .foregroundColor(SF.Colors.purple)
                                        .padding(.horizontal, 4)
                                        .padding(.vertical, 2)
                                        .background(SF.Colors.purple.opacity(0.1))
                                        .cornerRadius(4)
                                }
                            }
                            HStack(spacing: 5) {
                                if !displayRole.isEmpty {
                                    Text(displayRole)
                                        .font(.system(size: 11).italic())
                                        .foregroundColor(SF.Colors.textSecondary)
                                }
                                if let pat = currentPhasePattern, !pat.isEmpty {
                                    PatternBadge(pattern: pat)
                                }
                            }
                        }

                        Spacer()

                        if !event.toAgents.isEmpty {
                            eventRecipientsView(event.toAgents)
                        }

                        Text(event.timestamp, style: .time)
                            .font(.system(size: 9))
                            .foregroundColor(SF.Colors.textMuted)
                    }

                    Divider().background(SF.Colors.border.opacity(0.5))

                    if !event.data.isEmpty {
                        let cleanContent = Self.stripToolCallLines(event.data)
                        if !cleanContent.isEmpty {
                            MarkdownView(cleanContent, fontSize: 12)
                                .textSelection(.enabled)
                        }
                        let inlineTools = Self.extractInlineTools(event.data)
                        if !inlineTools.isEmpty {
                            HStack(spacing: 4) {
                                Image(systemName: "wrench.and.screwdriver")
                                    .font(.system(size: 9))
                                    .foregroundColor(SF.Colors.textMuted)
                                ForEach(Array(inlineTools.enumerated()), id: \.offset) { _, tool in
                                    HStack(spacing: 3) {
                                        Image(systemName: Self.toolIcon(tool))
                                            .font(.system(size: 9))
                                        Text(tool)
                                            .font(.system(size: 9, weight: .medium, design: .monospaced))
                                    }
                                    .foregroundColor(SF.Colors.textSecondary)
                                    .padding(.horizontal, 6)
                                    .padding(.vertical, 3)
                                    .background(SF.Colors.bgTertiary)
                                    .cornerRadius(4)
                                }
                                Spacer()
                            }
                        }
                    }

                    if isLast && isActive && bridge.isRunning {
                        HStack(spacing: 3) {
                            Circle().fill(SF.Colors.purple).frame(width: 4, height: 4)
                                .modifier(PulseAnimation())
                            Circle().fill(SF.Colors.purple).frame(width: 4, height: 4)
                                .modifier(PulseAnimation(delay: 0.2))
                            Circle().fill(SF.Colors.purple).frame(width: 4, height: 4)
                                .modifier(PulseAnimation(delay: 0.4))
                        }
                    }
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 12)
            }
            .background(SF.Colors.bgCard)
            .cornerRadius(10)
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .stroke(SF.Colors.border.opacity(0.5), lineWidth: 0.5)
            )
        )
    }

    // MARK: - Thinking Row

    func thinkingRow(_ event: SFBridge.AgentEvent) -> some View {
        let name = !event.agentName.isEmpty ? event.agentName : catalog.agentName(event.agentId)
        return HStack(spacing: 10) {
            AgentAvatarView(agentId: event.agentId, size: 28)
            ProgressView().controlSize(.small)
            Text("\(name) rédige…")
                .font(.system(size: 12))
                .foregroundColor(SF.Colors.textMuted)
            Spacer()
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 6)
    }

    // MARK: - Event Type Badge

    @ViewBuilder
    func eventTypeBadge(_ type: String) -> some View {
        let (bg, fg) = eventBadgeColors(type)
        Text(type.uppercased())
            .font(.system(size: 9, weight: .bold))
            .foregroundColor(fg)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(bg)
            .cornerRadius(4)
    }

    // MARK: - Tool Call Text Helpers

    /// Strip "[Calling tools: ...]" lines and standalone tool call text from content
    static func stripToolCallLines(_ text: String) -> String {
        text.components(separatedBy: "\n")
            .filter { line in
                let trimmed = line.trimmingCharacters(in: .whitespaces)
                if trimmed.hasPrefix("[Calling tools:") && trimmed.hasSuffix("]") { return false }
                if trimmed.hasPrefix("[Tool ") && trimmed.contains("result:") { return false }
                return true
            }
            .joined(separator: "\n")
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// Extract tool names from "[Calling tools: name(args), name2(args)]" lines
    static func extractInlineTools(_ text: String) -> [String] {
        var tools: [String] = []
        for line in text.components(separatedBy: "\n") {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            guard trimmed.hasPrefix("[Calling tools:"), trimmed.hasSuffix("]") else { continue }
            let inner = String(trimmed.dropFirst("[Calling tools:".count).dropLast())
                .trimmingCharacters(in: .whitespaces)
            var i = inner.startIndex
            while i < inner.endIndex {
                if let parenRange = inner.range(of: "(", range: i..<inner.endIndex) {
                    let beforeParen = inner[i..<parenRange.lowerBound]
                        .trimmingCharacters(in: .whitespaces)
                    let name = beforeParen.components(separatedBy: ",").last?
                        .trimmingCharacters(in: .whitespaces) ?? ""
                    if !name.isEmpty && name.allSatisfy({ $0.isLetter || $0 == "_" }) {
                        let short = name
                            .replacingOccurrences(of: "code_", with: "")
                            .replacingOccurrences(of: "file_", with: "")
                            .replacingOccurrences(of: "memory_", with: "mem:")
                            .replacingOccurrences(of: "git_", with: "git:")
                            .replacingOccurrences(of: "list_", with: "ls:")
                            .replacingOccurrences(of: "deep_", with: "")
                        tools.append(short)
                    }
                    var depth = 1
                    var j = parenRange.upperBound
                    while j < inner.endIndex && depth > 0 {
                        if inner[j] == "(" { depth += 1 }
                        if inner[j] == ")" { depth -= 1 }
                        j = inner.index(after: j)
                    }
                    i = j
                } else {
                    break
                }
            }
        }
        return tools
    }

    static func toolIcon(_ name: String) -> String {
        switch true {
        case name.contains("read"):   return "doc.text"
        case name.contains("write"):  return "square.and.pencil"
        case name.contains("edit"):   return "pencil"
        case name.contains("search"): return "magnifyingglass"
        case name.hasPrefix("ls:"):   return "list.bullet"
        case name.hasPrefix("git:"):  return "arrow.triangle.branch"
        case name.hasPrefix("mem:"):  return "brain"
        default:                       return "wrench"
        }
    }

    // MARK: - Event Recipients

    @ViewBuilder
    func eventRecipientsView(_ toAgents: [String]) -> some View {
        HStack(spacing: 3) {
            Image(systemName: "arrow.right")
                .font(.system(size: 9, weight: .medium))
                .foregroundColor(SF.Colors.textMuted)
            ForEach(toAgents, id: \.self) { agentId in
                let displayName = agentId == "all" ? "Tous" : catalog.agentName(agentId)
                let c = catalog.agentColor(agentId)
                HStack(spacing: 3) {
                    AgentAvatarView(agentId: agentId, size: 16)
                    Text(displayName)
                        .font(.system(size: 10, weight: .medium))
                        .foregroundColor(c)
                }
                .padding(.horizontal, 4)
                .padding(.vertical, 2)
                .background(c.opacity(0.1))
                .cornerRadius(4)
            }
        }
    }

    // MARK: - Event Color Helpers

    func eventBorderColor(_ type: String) -> Color {
        switch type {
        case "instruction", "delegation": return SF.Colors.yellowDeep
        case "response", "approval":      return Color(red: 0.13, green: 0.77, blue: 0.37)
        case "veto":                      return SF.Colors.error
        case "synthesis":                 return SF.Colors.po
        default:                          return SF.Colors.textMuted.opacity(0.5)
        }
    }

    func eventBadgeColors(_ type: String) -> (Color, Color) {
        switch type {
        case "instruction":  return (SF.Colors.yellowDeep.opacity(0.2), SF.Colors.yellowDeep)
        case "delegation":   return (SF.Colors.purple.opacity(0.2), SF.Colors.purple)
        case "approval":     return (Color(red: 0.13, green: 0.77, blue: 0.37).opacity(0.2), Color(red: 0.13, green: 0.77, blue: 0.37))
        case "veto":         return (SF.Colors.error.opacity(0.2), SF.Colors.error)
        case "synthesis":    return (SF.Colors.po.opacity(0.2), SF.Colors.po)
        default:             return (SF.Colors.textMuted.opacity(0.15), SF.Colors.textMuted)
        }
    }
}
