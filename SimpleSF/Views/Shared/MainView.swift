import SwiftUI

struct MainView: View {
    @EnvironmentObject var appState: AppState
    @State private var selection: SidebarItem = .projects

    var body: some View {
        NavigationSplitView {
            SidebarView(selection: $selection)
        } detail: {
            detailView(for: selection)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                ModePicker()
            }
        }
    }

    @ViewBuilder
    private func detailView(for item: SidebarItem) -> some View {
        switch item {
        case .projects:  ProjectsView()
        case .jarvis:    JarvisView()
        case .ideation:  IdeationView()
        case .portfolio: WebSFView(path: "/")
        case .pi:        WebSFView(path: "/pi")
        case .backlog:   WebSFView(path: "/backlog")
        case .art:       WebSFView(path: "/art")
        case .metrics:   WebSFView(path: "/metrics")
        case .live:      WebSFView(path: "/live")
        case .workflows: WebSFView(path: "/workflows")
        case .agents:    WebSFView(path: "/agents")
        }
    }
}

enum SidebarItem: String, Hashable {
    // Simple
    case projects, jarvis, ideation
    // Advanced
    case portfolio, pi, backlog, art, metrics, live, workflows, agents

    var label: String {
        switch self {
        case .projects:  return "Projects"
        case .jarvis:    return "Jarvis"
        case .ideation:  return "Ideation"
        case .portfolio: return "Portfolio"
        case .pi:        return "PI Board"
        case .backlog:   return "Backlog"
        case .art:       return "ART"
        case .metrics:   return "Metrics"
        case .live:      return "Live"
        case .workflows: return "Workflows"
        case .agents:    return "Agents"
        }
    }

    var icon: String {
        switch self {
        case .projects:  return "folder.fill"
        case .jarvis:    return "bubble.left.and.bubble.right.fill"
        case .ideation:  return "lightbulb.fill"
        case .portfolio: return "briefcase.fill"
        case .pi:        return "calendar.badge.checkmark"
        case .backlog:   return "list.bullet.rectangle"
        case .art:       return "person.3.fill"
        case .metrics:   return "chart.bar.fill"
        case .live:      return "antenna.radiowaves.left.and.right"
        case .workflows: return "arrow.triangle.2.circlepath"
        case .agents:    return "cpu.fill"
        }
    }
}

struct SidebarView: View {
    @EnvironmentObject var appState: AppState
    @Binding var selection: SidebarItem

    var body: some View {
        List(selection: $selection) {
            Section("Simple SF") {
                ForEach([SidebarItem.projects, .jarvis, .ideation], id: \.self) { item in
                    Label(item.label, systemImage: item.icon).tag(item)
                }
            }
            if appState.mode == .advanced {
                Section("Advanced") {
                    ForEach([SidebarItem.portfolio, .pi, .backlog, .art, .metrics, .live, .workflows, .agents], id: \.self) { item in
                        Label(item.label, systemImage: item.icon).tag(item)
                    }
                }
            }
        }
        .navigationSplitViewColumnWidth(min: 180, ideal: 200)
    }
}

struct ModePicker: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        Picker("", selection: Binding(
            get: { appState.mode },
            set: { appState.setMode($0) }
        )) {
            Label("Simple", systemImage: "square.grid.2x2").tag(SFMode.simple)
            Label("Advanced", systemImage: "gearshape.2").tag(SFMode.advanced)
        }
        .pickerStyle(.segmented)
        .help("Switch between Simple and Advanced mode")
    }
}
