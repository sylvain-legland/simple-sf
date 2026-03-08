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
    }

    @ViewBuilder
    private func detailView(for item: SidebarItem) -> some View {
        switch item {
        case .projects:  ProjectsView()
        case .jarvis:    JarvisView()
        case .ideation:  IdeationView()
        case .settings:  OnboardingView()
        }
    }
}

enum SidebarItem: String, Hashable {
    case projects, jarvis, ideation, settings

    var label: String {
        switch self {
        case .projects: return "Projects"
        case .jarvis:   return "Jarvis"
        case .ideation: return "Ideation"
        case .settings: return "API Keys"
        }
    }

    var icon: String {
        switch self {
        case .projects: return "folder.fill"
        case .jarvis:   return "sparkles"
        case .ideation: return "lightbulb.fill"
        case .settings: return "key.fill"
        }
    }
}

struct SidebarView: View {
    @Binding var selection: SidebarItem
    @ObservedObject private var llm = LLMService.shared

    var body: some View {
        List(selection: $selection) {
            Section {
                ForEach([SidebarItem.projects, .jarvis, .ideation, .settings], id: \.self) { item in
                    Label(item.label, systemImage: item.icon).tag(item)
                }
            } header: {
                HStack {
                    Image(systemName: "sparkles").foregroundColor(.purple)
                    Text("Simple SF").fontWeight(.semibold)
                }
            }
        }
        .navigationSplitViewColumnWidth(min: 180, ideal: 200)
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
