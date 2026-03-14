import SwiftUI

// Ref: FT-SSF-003

// MARK: - Projects View (accordion: card + inline discussion)

@MainActor
struct ProjectsView: View {
    @ObservedObject private var store = ProjectStore.shared
    @ObservedObject private var bridge = SFBridge.shared
    @State private var searchText = ""
    @State private var expandedProjectId: String?

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
            HStack(spacing: 10) {
                Image(systemName: "folder.fill")
                    .font(.system(size: 20))
                    .foregroundColor(SF.Colors.purple)
                Text("Projects")
                    .font(.system(size: 22, weight: .bold))
                    .foregroundColor(SF.Colors.textPrimary)
                Spacer()
                Text("\(store.projects.count) projects")
                    .font(.system(size: 12))
                    .foregroundColor(SF.Colors.textSecondary)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 4)
                    .background(SF.Colors.bgTertiary)
                    .cornerRadius(6)
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 16)

            if !store.projects.isEmpty {
                HStack(spacing: 8) {
                    Image(systemName: "magnifyingglass")
                        .font(.system(size: 12))
                        .foregroundColor(SF.Colors.textMuted)
                    TextField("Search projects…", text: $searchText)
                        .textFieldStyle(.plain)
                        .font(.system(size: 13))
                        .foregroundColor(SF.Colors.textPrimary)
                }
                .padding(10)
                .background(SF.Colors.bgTertiary)
                .cornerRadius(SF.Radius.md)
                .overlay(RoundedRectangle(cornerRadius: SF.Radius.md).stroke(SF.Colors.border, lineWidth: 1))
                .padding(.horizontal, 24)
            }

            Divider().background(SF.Colors.border).padding(.top, 10)

            ScrollView {
                LazyVStack(spacing: 12) {
                    if !store.projects.isEmpty {
                        ForEach(filtered) { project in
                            ProjectAccordion(
                                project: project,
                                isExpanded: expandedProjectId == project.id,
                                toggle: { toggleExpand(project.id) }
                            )
                        }
                    } else {
                        emptyState
                    }
                }
                .padding(24)

                pilotSection
            }
        }
        .background(SF.Colors.bgPrimary)
    }

    private func toggleExpand(_ id: String) {
        withAnimation(.easeInOut(duration: 0.25)) {
            expandedProjectId = expandedProjectId == id ? nil : id
        }
    }

    // MARK: - Pilot Projects Section

    private var pilotSection: some View {
        VStack(spacing: 0) {
            Divider().background(SF.Colors.border)

            HStack(spacing: 10) {
                Image(systemName: "flag.fill")
                    .font(.system(size: 14))
                    .foregroundColor(SF.Colors.accent)
                Text("Projets Pilotes")
                    .font(.system(size: 16, weight: .bold))
                    .foregroundColor(SF.Colors.textPrimary)
                Text("AC validation")
                    .font(.system(size: 11))
                    .foregroundColor(SF.Colors.textMuted)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 2)
                    .background(SF.Colors.accent.opacity(0.15))
                    .cornerRadius(4)
                Spacer()
                Button {
                    loadPilotProjects()
                } label: {
                    Label("Charger", systemImage: "plus.circle.fill")
                        .font(.system(size: 12, weight: .medium))
                        .foregroundColor(SF.Colors.purple)
                }
                .buttonStyle(.plain)

                Button {
                    resetPilotProjects()
                } label: {
                    Label("Réinitialiser", systemImage: "arrow.counterclockwise")
                        .font(.system(size: 12, weight: .medium))
                        .foregroundColor(SF.Colors.error)
                }
                .buttonStyle(.plain)
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 12)

            let pilots = store.projects.filter { p in
                pilotProjects.contains { $0.name == p.name }
            }
            if !pilots.isEmpty {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 10) {
                        ForEach(pilots) { project in
                            pilotCard(project)
                        }
                    }
                    .padding(.horizontal, 24)
                    .padding(.bottom, 16)
                }
            } else {
                HStack(spacing: 8) {
                    Image(systemName: "info.circle")
                        .foregroundColor(SF.Colors.textMuted)
                    Text("Cliquez \"Charger\" pour importer les 8 projets pilotes")
                        .font(.system(size: 12))
                        .foregroundColor(SF.Colors.textMuted)
                }
                .padding(.horizontal, 24)
                .padding(.bottom, 16)
            }
        }
    }

    private func pilotCard(_ project: Project) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(project.name)
                .font(.system(size: 12, weight: .bold))
                .foregroundColor(SF.Colors.textPrimary)
                .lineLimit(1)
            Text(project.tech)
                .font(.system(size: 10))
                .foregroundColor(SF.Colors.purple)
                .lineLimit(1)
            Text(project.description)
                .font(.system(size: 10))
                .foregroundColor(SF.Colors.textSecondary)
                .lineLimit(2)
            HStack(spacing: 4) {
                Circle()
                    .fill(Color(hex: UInt(project.status.color.dropFirst(), radix: 16) ?? 0x6366f1))
                    .frame(width: 6, height: 6)
                Text(project.status.displayName)
                    .font(.system(size: 9, weight: .medium))
                    .foregroundColor(SF.Colors.textMuted)
            }
        }
        .frame(width: 180)
        .padding(12)
        .background(SF.Colors.bgCard)
        .cornerRadius(SF.Radius.md)
        .overlay(RoundedRectangle(cornerRadius: SF.Radius.md).stroke(SF.Colors.border, lineWidth: 0.5))
    }

    private func loadPilotProjects() {
        for pilot in pilotProjects {
            let exists = store.projects.contains { $0.name == pilot.name }
            if !exists {
                let project = Project(
                    name: pilot.name,
                    description: pilot.description,
                    tech: pilot.tech,
                    status: .idea
                )
                store.add(project)
            }
        }
    }

    private func resetPilotProjects() {
        let pilotNames = Set(pilotProjects.map(\.name))
        let toDelete = store.projects.filter { pilotNames.contains($0.name) }
        for p in toDelete {
            store.delete(p.id)
        }
    }

    private var emptyState: some View {
        VStack(spacing: 16) {
            Image(systemName: "sparkles")
                .font(.system(size: 48))
                .foregroundColor(SF.Colors.purple.opacity(0.4))
            Text("No projects yet")
                .font(.system(size: 18, weight: .semibold))
                .foregroundColor(SF.Colors.textSecondary)
            Text("Ask Jarvis to create a project for you.\n\"Create a project called MyApp using Swift\"")
                .font(.system(size: 13))
                .foregroundColor(SF.Colors.textMuted)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
