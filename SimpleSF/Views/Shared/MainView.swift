import SwiftUI

// Ref: FT-SSF-001, FT-SSF-015
struct MainView: View {
    @State private var selection: SidebarItem = .jarvis
    @ObservedObject private var l10n = L10n.shared

    var body: some View {
        NavigationSplitView {
            SidebarView(selection: $selection)
        } detail: {
            detailView(for: selection)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(SF.Colors.bgPrimary)
        }
        .navigationSplitViewStyle(.balanced)
        .environment(\.layoutDirection, l10n.layoutDirection)
    }

    @ViewBuilder
    private func detailView(for item: SidebarItem) -> some View {
        switch item {
        case .jarvis:    JarvisView()
        case .projects:  ProjectsView()
        case .ideation:  IdeationView()
        case .settings:  OnboardingView()
        }
    }
}

// MARK: - Sidebar

enum SidebarItem: String, Hashable {
    case jarvis, projects, ideation, settings

    @MainActor
    var label: String {
        switch self {
        case .jarvis:    return L10n.shared.t(.navJarvis)
        case .projects:  return L10n.shared.t(.navProjects)
        case .ideation:  return L10n.shared.t(.navIdeation)
        case .settings:  return L10n.shared.t(.navSettings)
        }
    }

    var icon: String {
        switch self {
        case .jarvis:    return "sparkles"
        case .projects:  return "folder.fill"
        case .ideation:  return "lightbulb.fill"
        case .settings:  return "gearshape.fill"
        }
    }
}

struct SidebarView: View {
    @Binding var selection: SidebarItem
    @ObservedObject private var llm = LLMService.shared
    @ObservedObject private var bridge = SFBridge.shared
    @ObservedObject private var appState = AppState.shared
    @ObservedObject private var l10n = L10n.shared

    var body: some View {
        List(selection: $selection) {
            ForEach([SidebarItem.jarvis, .projects, .ideation, .settings], id: \.self) { item in
                HStack(spacing: 8) {
                    Label(item.label, systemImage: item.icon)
                        .font(.system(size: 13))
                        .tag(item)
                    if item == .projects && bridge.isRunning {
                        Spacer()
                        ProgressView().scaleEffect(0.5)
                    }
                }
            }
        }
        .navigationSplitViewColumnWidth(min: 160, ideal: 190)
        .safeAreaInset(edge: .top) {
            HStack(spacing: 6) {
                Image(systemName: "hammer.fill")
                    .foregroundColor(SF.Colors.purple)
                    .font(.system(size: 11))
                Text(l10n.t(.navSoftwareFactory))
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundColor(SF.Colors.textSecondary)
                Spacer()
                StatusDot(active: bridge.engineReady, size: 7)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
        }
        .safeAreaInset(edge: .bottom) {
            VStack(spacing: 8) {
                yoloToggle
                providerBadge
            }
        }
    }

    private var yoloToggle: some View {
        HStack(spacing: 6) {
            Image(systemName: appState.yoloMode ? "bolt.fill" : "bolt.slash")
                .font(.system(size: 10))
                .foregroundColor(appState.yoloMode ? SF.Colors.warning : SF.Colors.textMuted)
            Text(l10n.t(.sidebarYolo))
                .font(.system(size: 10, weight: .bold, design: .monospaced))
                .foregroundColor(appState.yoloMode ? SF.Colors.warning : SF.Colors.textMuted)
            Spacer()
            Toggle("", isOn: Binding(
                get: { appState.yoloMode },
                set: { appState.setYoloMode($0) }
            ))
            .toggleStyle(.switch)
            .controlSize(.mini)
            .labelsHidden()
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 4)
        .help(l10n.t(.sidebarYoloHelp))
    }

    private var providerBadge: some View {
        let provider = llm.activeProvider
        let isActive = provider != nil

        return VStack(spacing: 0) {
            Divider().opacity(0.3)
            HStack(spacing: 8) {
                // Provider icon
                Image(systemName: providerIcon(provider))
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundColor(isActive ? providerColor(provider) : SF.Colors.textMuted)
                    .frame(width: 16)

                VStack(alignment: .leading, spacing: 1) {
                    Text(provider?.displayName ?? l10n.t(.sidebarNoLLM))
                        .font(.system(size: 10, weight: .bold))
                        .foregroundColor(isActive ? SF.Colors.textPrimary : SF.Colors.warning)
                    Text(activeModelShort(provider))
                        .font(.system(size: 9, weight: .medium, design: .monospaced))
                        .foregroundColor(isActive ? SF.Colors.textMuted : SF.Colors.error)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }

                Spacer()

                // Status dot
                Circle()
                    .fill(isActive ? SF.Colors.success : SF.Colors.error)
                    .frame(width: 6, height: 6)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
        }
    }

    private func providerIcon(_ provider: LLMProvider?) -> String {
        switch provider {
        case .mlx:        return "cpu"
        case .ollama:     return "desktopcomputer"
        case .openai:     return "sparkle"
        case .anthropic:  return "brain.head.profile"
        case .gemini:     return "diamond.fill"
        case .minimax:    return "bolt.fill"
        case .openrouter: return "network"
        case .glm:        return "globe.asia.australia.fill"
        case .alibaba:    return "cloud.fill"
        case .kimi:       return "moon.fill"
        case .none:       return "exclamationmark.triangle"
        }
    }

    private func providerColor(_ provider: LLMProvider?) -> Color {
        switch provider {
        case .mlx:        return SF.Colors.purple
        case .ollama:     return SF.Colors.info
        case .openai:     return SF.Colors.success
        case .anthropic:  return SF.Colors.warning
        case .minimax:    return SF.Colors.accent
        default:          return SF.Colors.textSecondary
        }
    }

    private func activeModelShort(_ provider: LLMProvider?) -> String {
        guard let provider else { return l10n.t(.statusNotConfigured) }
        switch provider {
        case .mlx:
            let name = MLXService.shared.activeModel?.name ?? "loading…"
            return name.split(separator: "/").last.map(String.init) ?? name
        case .ollama:
            return OllamaService.shared.activeModel?.name ?? "loading…"
        default:
            let sel = AppState.shared.selectedModel
            return sel.isEmpty ? provider.defaultModel : sel
        }
    }
}
