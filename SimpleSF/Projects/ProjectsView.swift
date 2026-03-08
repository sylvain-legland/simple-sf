import SwiftUI

@MainActor
struct ProjectsView: View {
    @ObservedObject private var store = ProjectStore.shared
    @State private var searchText = ""

    private var filtered: [Project] {
        guard !searchText.isEmpty else { return store.projects }
        return store.projects.filter {
            $0.name.localizedCaseInsensitiveContains(searchText) ||
            $0.description.localizedCaseInsensitiveContains(searchText)
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Image(systemName: "folder.fill")
                    .foregroundColor(.purple)
                Text("Projects")
                    .font(.title2.bold())
                Spacer()
                Text("\(store.projects.count) projects")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .padding()

            if !store.projects.isEmpty {
                // Search
                HStack {
                    Image(systemName: "magnifyingglass")
                        .foregroundColor(.secondary)
                    TextField("Search projects…", text: $searchText)
                        .textFieldStyle(.plain)
                }
                .padding(8)
                .background(Color(.controlBackgroundColor))
                .cornerRadius(8)
                .padding(.horizontal)
            }

            Divider().padding(.top, 8)

            if store.projects.isEmpty {
                emptyState
            } else {
                projectList
            }
        }
    }

    private var projectList: some View {
        ScrollView {
            LazyVStack(spacing: 8) {
                ForEach(filtered) { project in
                    ProjectCard(project: project)
                }
            }
            .padding()
        }
    }

    private var emptyState: some View {
        VStack(spacing: 16) {
            Image(systemName: "sparkles")
                .font(.system(size: 48))
                .foregroundColor(.purple.opacity(0.4))
            Text("No projects yet")
                .font(.title3)
                .foregroundColor(.secondary)
            Text("Ask Jarvis to create a project for you.\n\"Create a project called MyApp using Swift\"")
                .font(.caption)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

struct ProjectCard: View {
    let project: Project

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Circle()
                    .fill(Color(hex: project.status.color))
                    .frame(width: 10, height: 10)
                Text(project.name)
                    .font(.headline)
                Spacer()
                Text(project.status.displayName)
                    .font(.caption)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 3)
                    .background(Color(hex: project.status.color).opacity(0.2))
                    .cornerRadius(4)
            }
            if !project.description.isEmpty {
                Text(project.description)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(2)
            }
            if project.progress > 0 {
                ProgressView(value: project.progress)
                    .tint(Color(hex: project.status.color))
            }
            HStack {
                if !project.tech.isEmpty {
                    Label(project.tech, systemImage: "chevron.left.forwardslash.chevron.right")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
                Spacer()
            }
        }
        .padding(12)
        .background(Color(.controlBackgroundColor))
        .cornerRadius(10)
    }
}

extension Color {
    init(hex: String) {
        let hex = hex.trimmingCharacters(in: CharacterSet.alphanumerics.inverted)
        var int: UInt64 = 0
        Scanner(string: hex).scanHexInt64(&int)
        let r = Double((int >> 16) & 0xFF) / 255.0
        let g = Double((int >> 8) & 0xFF) / 255.0
        let b = Double(int & 0xFF) / 255.0
        self.init(red: r, green: g, blue: b)
    }
}
