import SwiftUI

@MainActor
struct ProjectsView: View {
    @ObservedObject private var store = ProjectStore.shared
    @State private var showCreate = false
    @State private var selectedProject: Project?
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
                Button(action: { showCreate = true }) {
                    Image(systemName: "plus.circle.fill")
                        .font(.title3)
                        .foregroundColor(.purple)
                }
                .buttonStyle(.plain)
            }
            .padding()

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

            Divider().padding(.top, 8)

            if store.projects.isEmpty {
                emptyState
            } else {
                projectList
            }
        }
        .sheet(isPresented: $showCreate) {
            CreateProjectView()
        }
        .sheet(item: $selectedProject) { proj in
            ProjectDetailView(project: proj)
        }
    }

    private var projectList: some View {
        ScrollView {
            LazyVStack(spacing: 8) {
                ForEach(filtered) { project in
                    ProjectCard(project: project)
                        .onTapGesture { selectedProject = project }
                }
            }
            .padding()
        }
    }

    private var emptyState: some View {
        VStack(spacing: 16) {
            Image(systemName: "folder.badge.plus")
                .font(.system(size: 48))
                .foregroundColor(.purple.opacity(0.4))
            Text("No projects yet")
                .font(.title3)
                .foregroundColor(.secondary)
            Button("Create your first project") { showCreate = true }
                .buttonStyle(.bordered)
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
                Text(project.updatedAt, style: .relative)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
        .padding(12)
        .background(Color(.controlBackgroundColor))
        .cornerRadius(10)
    }
}

struct CreateProjectView: View {
    @Environment(\.dismiss) var dismiss
    @State private var name = ""
    @State private var description = ""
    @State private var tech = ""

    var body: some View {
        VStack(spacing: 16) {
            Text("New Project")
                .font(.title2.bold())
            Form {
                TextField("Name", text: $name)
                TextField("Description", text: $description)
                TextField("Tech stack (e.g. Swift, Python)", text: $tech)
            }
            .formStyle(.grouped)
            HStack {
                Button("Cancel") { dismiss() }
                    .keyboardShortcut(.cancelAction)
                Spacer()
                Button("Create") {
                    ProjectStore.shared.add(Project(name: name, description: description, tech: tech))
                    dismiss()
                }
                .buttonStyle(.borderedProminent)
                .disabled(name.isEmpty)
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding()
        .frame(width: 440)
    }
}

struct ProjectDetailView: View {
    @Environment(\.dismiss) var dismiss
    @State var project: Project

    var body: some View {
        VStack(spacing: 16) {
            Text(project.name)
                .font(.title2.bold())
            Form {
                TextField("Name", text: $project.name)
                TextField("Description", text: $project.description)
                TextField("Tech stack", text: $project.tech)
                Picker("Status", selection: $project.status) {
                    ForEach(ProjectStatus.allCases, id: \.self) {
                        Text($0.displayName).tag($0)
                    }
                }
                Slider(value: $project.progress, in: 0...1) {
                    Text("Progress \(Int(project.progress * 100))%")
                }
            }
            .formStyle(.grouped)
            HStack {
                Button("Delete", role: .destructive) {
                    ProjectStore.shared.delete(project.id)
                    dismiss()
                }
                Spacer()
                Button("Done") {
                    ProjectStore.shared.update(project)
                    dismiss()
                }
                .buttonStyle(.borderedProminent)
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding()
        .frame(width: 480)
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
