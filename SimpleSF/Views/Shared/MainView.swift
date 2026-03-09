import SwiftUI

struct MainView: View {
    @State private var selection: SidebarItem = .jarvis

    var body: some View {
        NavigationSplitView {
            SidebarView(selection: $selection)
        } detail: {
            detailView(for: selection)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(SF.Colors.bgPrimary)
        }
        .navigationSplitViewStyle(.balanced)
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

    var label: String {
        switch self {
        case .jarvis:    return "Jarvis"
        case .projects:  return "Projects"
        case .ideation:  return "Ideation"
        case .settings:  return "Settings"
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
                Text("Software Factory")
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
            Text("YOLO")
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
        .help("YOLO: auto-approve all GO/NOGO gates")
    }

    private var providerBadge: some View {
        HStack(spacing: 6) {
            StatusDot(active: llm.activeProvider != nil, size: 7)
            Text(llm.activeDisplayName)
                .font(.system(size: 11, weight: .medium))
                .foregroundColor(llm.activeProvider != nil ? SF.Colors.textSecondary : SF.Colors.warning)
                .lineLimit(1)
        }
        .padding(12)
    }
}
