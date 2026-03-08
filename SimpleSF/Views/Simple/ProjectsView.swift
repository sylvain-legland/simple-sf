import SwiftUI

struct ProjectsView: View {
    @State private var projects: [SFProject] = []
    @State private var isLoading = true
    @State private var error: String?

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Projects").font(.title2.bold())
                Spacer()
                Button { Task { await load() } } label: {
                    Label("Refresh", systemImage: "arrow.clockwise")
                }
                Button { /* new project sheet */ } label: {
                    Label("New Project", systemImage: "plus")
                }.buttonStyle(.borderedProminent)
            }
            .padding(.horizontal, 24).padding(.vertical, 16)

            Divider()

            if isLoading {
                Spacer()
                ProgressView("Loading projects…")
                Spacer()
            } else if let error {
                Spacer()
                ContentUnavailableView("Error", systemImage: "exclamationmark.triangle",
                                       description: Text(error))
                Spacer()
            } else if projects.isEmpty {
                Spacer()
                ContentUnavailableView("No projects yet",
                                       systemImage: "folder.badge.plus",
                                       description: Text("Create your first project to get started."))
                Spacer()
            } else {
                ScrollView {
                    LazyVStack(spacing: 12) {
                        ForEach(projects) { project in
                            ProjectRowView(project: project) { action in
                                await handleAction(action, project: project)
                            }
                        }
                    }
                    .padding(24)
                }
            }
        }
        .task { await load() }
    }

    private func load() async {
        isLoading = true
        error = nil
        do {
            let result = try await SFClient.shared.get("/api/projects") as [SFProject]
            projects = result
        } catch {
            self.error = error.localizedDescription
        }
        isLoading = false
    }

    private func handleAction(_ action: ProjectAction, project: SFProject) async {
        switch action {
        case .start:
            _ = try? await SFClient.shared.postRaw("/api/missions/\(project.id)/start", body: [:])
        case .pause:
            _ = try? await SFClient.shared.postRaw("/api/missions/\(project.id)/pause", body: [:])
        case .stop:
            _ = try? await SFClient.shared.postRaw("/api/missions/\(project.id)/stop", body: [:])
        }
        await load()
    }
}

enum ProjectAction { case start, pause, stop }

struct ProjectRowView: View {
    let project: SFProject
    let onAction: (ProjectAction) async -> Void

    @State private var isActing = false

    var body: some View {
        HStack(spacing: 16) {
            // Status dot
            Circle()
                .fill(statusColor)
                .frame(width: 10, height: 10)
                .overlay {
                    if project.isRunning {
                        Circle().stroke(statusColor.opacity(0.4), lineWidth: 3)
                            .scaleEffect(1.6)
                    }
                }

            // Info
            VStack(alignment: .leading, spacing: 4) {
                Text(project.name).font(.headline)
                Text(project.description ?? "").font(.caption).foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer()

            // Progress
            VStack(alignment: .trailing, spacing: 4) {
                Text("\(Int(project.progress * 100))%").font(.caption.monospacedDigit())
                ProgressView(value: project.progress)
                    .progressViewStyle(.linear)
                    .frame(width: 120)
                    .tint(statusColor)
            }

            // Spinner if running
            if project.isRunning {
                ProgressView()
                    .scaleEffect(0.8)
                    .frame(width: 20)
            }

            // Action buttons
            HStack(spacing: 6) {
                if project.isRunning {
                    ActionButton(icon: "pause.fill", help: "Pause") {
                        isActing = true
                        await onAction(.pause)
                        isActing = false
                    }
                    ActionButton(icon: "stop.fill", help: "Stop", tint: .red) {
                        isActing = true
                        await onAction(.stop)
                        isActing = false
                    }
                } else {
                    ActionButton(icon: "play.fill", help: "Start", tint: .green) {
                        isActing = true
                        await onAction(.start)
                        isActing = false
                    }
                }
            }
            .disabled(isActing)
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 12).fill(.secondary.opacity(0.06)))
        .overlay(RoundedRectangle(cornerRadius: 12).stroke(.secondary.opacity(0.15)))
    }

    private var statusColor: Color {
        switch project.status {
        case "running", "active": return .green
        case "paused":            return .orange
        case "failed":            return .red
        case "completed":         return .blue
        default:                  return .secondary
        }
    }
}

struct ActionButton: View {
    let icon: String
    let help: String
    var tint: Color = .accentColor
    let action: () async -> Void
    @State private var busy = false

    var body: some View {
        Button {
            busy = true
            Task { await action(); busy = false }
        } label: {
            Image(systemName: busy ? "circle" : icon)
                .frame(width: 28, height: 28)
        }
        .buttonStyle(.borderless)
        .foregroundStyle(tint)
        .help(help)
        .disabled(busy)
    }
}

// MARK: - Model

struct SFProject: Identifiable, Decodable {
    let id: String
    let name: String
    let description: String?
    let status: String
    let progress: Double
    var isRunning: Bool { status == "running" || status == "active" }

    enum CodingKeys: String, CodingKey {
        case id, name, description, status
        case progress = "completion_pct"
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(String.self, forKey: .id)
        name = try c.decode(String.self, forKey: .name)
        description = try? c.decode(String.self, forKey: .description)
        status = (try? c.decode(String.self, forKey: .status)) ?? "pending"
        let raw = (try? c.decode(Double.self, forKey: .progress)) ?? 0
        progress = raw / 100.0
    }
}
