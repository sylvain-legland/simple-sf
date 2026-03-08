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
    }

    @ViewBuilder
    private func detailView(for item: SidebarItem) -> some View {
        switch item {
        case .jarvis:    JarvisView()
        case .projects:  ProjectsView()
        case .ideation:  IdeationView()
        case .teams:     MissionView()
        case .settings:  OnboardingView()
        }
    }
}

// MARK: - Sidebar

enum SidebarItem: String, Hashable {
    case jarvis, projects, ideation, teams, settings

    var label: String {
        switch self {
        case .jarvis:    return "Jarvis"
        case .projects:  return "Projects"
        case .ideation:  return "Ideation"
        case .teams:     return "Value Stream"
        case .settings:  return "Settings"
        }
    }

    var icon: String {
        switch self {
        case .jarvis:    return "sparkles"
        case .projects:  return "folder.fill"
        case .ideation:  return "lightbulb.fill"
        case .teams:     return "flowchart.fill"
        case .settings:  return "gearshape.fill"
        }
    }
}

struct SidebarView: View {
    @Binding var selection: SidebarItem
    @ObservedObject private var llm = LLMService.shared
    @ObservedObject private var bridge = SFBridge.shared

    var body: some View {
        List(selection: $selection) {
            Section {
                ForEach([SidebarItem.jarvis, .projects, .ideation, .teams], id: \.self) { item in
                    HStack(spacing: 8) {
                        Label(item.label, systemImage: item.icon)
                            .font(.system(size: 13))
                            .tag(item)
                        if item == .teams && bridge.isRunning {
                            Spacer()
                            ProgressView().scaleEffect(0.5)
                        }
                    }
                }
            } header: {
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
            }

            Section {
                Label(SidebarItem.settings.label, systemImage: SidebarItem.settings.icon)
                    .font(.system(size: 13))
                    .tag(SidebarItem.settings)
            }
        }
        .navigationSplitViewColumnWidth(min: 160, ideal: 190)
        .safeAreaInset(edge: .bottom) {
            providerBadge
        }
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
