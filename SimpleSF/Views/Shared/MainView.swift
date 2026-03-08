import SwiftUI

struct MainView: View {
    @State private var selection: SidebarItem = .projects

    var body: some View {
        NavigationSplitView {
            SidebarView(selection: $selection)
        } detail: {
            detailView(for: selection)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    @ViewBuilder
    private func detailView(for item: SidebarItem) -> some View {
        switch item {
        case .projects:  ProjectsView()
        case .jarvis:    JarvisView()
        case .ideation:  IdeationView()
        case .missions:  MissionView()
        case .agents:    AgentsView()
        case .settings:  OnboardingView()
        }
    }
}

// MARK: - Sidebar

enum SidebarItem: String, Hashable {
    case projects, jarvis, ideation, missions, agents, settings

    var label: String {
        switch self {
        case .projects:  return "Projects"
        case .jarvis:    return "Jarvis"
        case .ideation:  return "Ideation"
        case .missions:  return "Missions"
        case .agents:    return "Agents"
        case .settings:  return "Settings"
        }
    }

    var icon: String {
        switch self {
        case .projects:  return "folder.fill"
        case .jarvis:    return "sparkles"
        case .ideation:  return "lightbulb.fill"
        case .missions:  return "play.circle.fill"
        case .agents:    return "person.3.fill"
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
                ForEach([SidebarItem.projects, .jarvis, .ideation], id: \.self) { item in
                    Label(item.label, systemImage: item.icon).tag(item)
                }
            } header: {
                HStack {
                    Image(systemName: "sparkles").foregroundColor(.purple)
                    Text("Simple").fontWeight(.semibold)
                }
            }

            Section {
                ForEach([SidebarItem.missions, .agents], id: \.self) { item in
                    HStack {
                        Label(item.label, systemImage: item.icon).tag(item)
                        if item == .missions && bridge.isRunning {
                            Spacer()
                            ProgressView().scaleEffect(0.5)
                        }
                    }
                }
            } header: {
                HStack {
                    Image(systemName: "cpu").foregroundColor(.purple)
                    Text("Factory").fontWeight(.semibold)
                    Spacer()
                    Circle()
                        .fill(bridge.engineReady ? Color.green : Color.gray)
                        .frame(width: 7, height: 7)
                }
            }

            Section {
                Label(SidebarItem.settings.label, systemImage: SidebarItem.settings.icon)
                    .tag(SidebarItem.settings)
            }
        }
        .navigationSplitViewColumnWidth(min: 180, ideal: 210)
        .safeAreaInset(edge: .bottom) {
            providerBadge
        }
    }

    private var providerBadge: some View {
        HStack(spacing: 6) {
            Circle()
                .fill(llm.activeProvider != nil ? Color.green : Color.gray)
                .frame(width: 7, height: 7)
            if let prov = llm.activeProvider {
                Text(prov.displayName)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            } else {
                Text("No LLM configured")
                    .font(.caption2)
                    .foregroundColor(.orange)
            }
        }
        .padding(10)
    }
}
