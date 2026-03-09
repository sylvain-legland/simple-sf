import SwiftUI

// MARK: - 14-phase SAFe product lifecycle (matches SF legacy Value Stream)

private let safePhases: [(name: String, short: String, pattern: String)] = [
    ("Idéation",            "Idéation",       "network"),
    ("Comité Stratégique",  "Comité Strat.",   "human-in-the-loop"),
    ("Constitution",        "Constitution",    "sequential"),
    ("Architecture",        "Architecture",    "aggregator"),
    ("Design System",       "Design Sys.",     "sequential"),
    ("Sprints Dev",         "Sprints Dev",     "hierarchical"),
    ("Build & Verify",      "Build & Verify",  "sequential"),
    ("Pipeline CI/CD",      "Pipeline CI",     "sequential"),
    ("Revue UX",            "Revue UX",        "loop"),
    ("Campagne QA",         "Campagne QA",     "loop"),
    ("Exécution Tests",     "Exécution",       "parallel"),
    ("Deploy Prod",         "Deploy Prod",     "human-in-the-loop"),
    ("Routage TMA",         "Routage",         "router"),
    ("Correctif & TMA",     "Correctif",       "loop"),
]

// Derive simulated phase progress from project status
private func simulatedActivePhase(for status: ProjectStatus, progress: Double) -> Int {
    switch status {
    case .idea:     return 0
    case .planning: return 1
    case .active:   return max(2, min(13, Int(progress * 13.0)))
    case .paused:   return max(2, min(13, Int(progress * 13.0)))
    case .done:     return 14 // all completed
    }
}

@MainActor
struct ProjectsView: View {
    @ObservedObject private var store = ProjectStore.shared
    @ObservedObject private var bridge = SFBridge.shared
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

            if store.projects.isEmpty {
                emptyState
            } else {
                projectList
            }
        }
        .background(SF.Colors.bgPrimary)
    }

    private var projectList: some View {
        ScrollView {
            LazyVStack(spacing: 12) {
                ForEach(filtered) { project in
                    ProjectCard(project: project)
                }
            }
            .padding(24)
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

// MARK: - Project Card with Phase Timeline + Controls

struct ProjectCard: View {
    let project: Project
    @ObservedObject private var bridge = SFBridge.shared

    private var activePhase: Int {
        simulatedActivePhase(for: project.status, progress: project.progress)
    }

    private var isActive: Bool { project.status == .active }
    private var isPaused: Bool { project.status == .paused }
    private var isQueued: Bool { project.status == .planning }
    private var isDone: Bool { project.status == .done }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // ── Row 1: Name + tech + status + controls
            HStack(spacing: 10) {
                Text(project.name)
                    .font(.system(size: 17, weight: .bold))
                    .foregroundColor(SF.Colors.textPrimary)

                if !project.tech.isEmpty {
                    Text(project.tech)
                        .font(.system(size: 11, weight: .medium))
                        .foregroundColor(SF.Colors.textMuted)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(SF.Colors.bgTertiary)
                        .cornerRadius(5)
                }

                Spacer()

                // Live status indicator
                statusIndicator

                // Play / Pause / Stop buttons
                controlButtons
            }

            // ── Description
            if !project.description.isEmpty {
                Text(project.description)
                    .font(.system(size: 13))
                    .foregroundColor(SF.Colors.textSecondary)
                    .lineLimit(2)
            }

            // ── Global progress bar (full width)
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 6) {
                    Image(systemName: "flowchart.fill")
                        .font(.system(size: 11))
                        .foregroundColor(SF.Colors.purple.opacity(0.7))
                    Text("Cycle de Vie Produit Complet")
                        .font(.system(size: 11, weight: .medium))
                        .foregroundColor(SF.Colors.textSecondary)
                    Spacer()
                    Text("\(activePhase)/14 phases")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundColor(SF.Colors.textSecondary)
                }
                GeometryReader { geo in
                    ZStack(alignment: .leading) {
                        RoundedRectangle(cornerRadius: 3)
                            .fill(SF.Colors.bgTertiary)
                            .frame(height: 6)
                        RoundedRectangle(cornerRadius: 3)
                            .fill(isDone ? SF.Colors.success : SF.Colors.purple)
                            .frame(width: geo.size.width * CGFloat(activePhase) / 14.0, height: 6)
                    }
                }
                .frame(height: 6)
            }

            // ── Phase timeline (bigger dots)
            MiniPhaseTimeline(activePhase: activePhase, projectDone: isDone)
        }
        .padding(16)
        .background(SF.Colors.bgCard)
        .cornerRadius(SF.Radius.lg)
        .overlay(
            RoundedRectangle(cornerRadius: SF.Radius.lg)
                .stroke(isActive ? SF.Colors.purple.opacity(0.5) : SF.Colors.border, lineWidth: isActive ? 1.5 : 1)
        )
    }

    // ── Status indicator: spinner / queued / done
    @ViewBuilder
    private var statusIndicator: some View {
        if isActive {
            HStack(spacing: 5) {
                ProgressView().scaleEffect(0.55)
                Text("Agents en cours…")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundColor(SF.Colors.purple)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(SF.Colors.purple.opacity(0.1))
            .cornerRadius(6)
        } else if isQueued {
            HStack(spacing: 5) {
                Image(systemName: "clock.fill")
                    .font(.system(size: 10))
                Text("Queued")
                    .font(.system(size: 11, weight: .semibold))
            }
            .foregroundColor(SF.Colors.warning)
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(SF.Colors.warning.opacity(0.1))
            .cornerRadius(6)
        } else if isPaused {
            HStack(spacing: 5) {
                Image(systemName: "pause.circle.fill")
                    .font(.system(size: 10))
                Text("Paused")
                    .font(.system(size: 11, weight: .semibold))
            }
            .foregroundColor(SF.Colors.warning)
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(SF.Colors.warning.opacity(0.1))
            .cornerRadius(6)
        } else if isDone {
            HStack(spacing: 5) {
                Image(systemName: "checkmark.circle.fill")
                    .font(.system(size: 10))
                Text("Terminé")
                    .font(.system(size: 11, weight: .semibold))
            }
            .foregroundColor(SF.Colors.success)
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(SF.Colors.success.opacity(0.1))
            .cornerRadius(6)
        } else {
            Text(project.status.displayName)
                .font(.system(size: 11, weight: .semibold))
                .foregroundColor(Color(hex: project.status.color))
                .padding(.horizontal, 10)
                .padding(.vertical, 5)
                .background(Color(hex: project.status.color).opacity(0.12))
                .cornerRadius(6)
        }
    }

    // ── Play / Pause / Stop (video-style)
    private var controlButtons: some View {
        HStack(spacing: 4) {
            if isActive {
                // Pause
                Button(action: { ProjectStore.shared.setStatus(project.id, status: .paused) }) {
                    Image(systemName: "pause.fill")
                        .font(.system(size: 12))
                        .foregroundColor(SF.Colors.warning)
                        .frame(width: 28, height: 28)
                        .background(SF.Colors.warning.opacity(0.12))
                        .cornerRadius(6)
                }
                .buttonStyle(.plain)
                // Stop
                Button(action: { ProjectStore.shared.setStatus(project.id, status: .idea) }) {
                    Image(systemName: "stop.fill")
                        .font(.system(size: 12))
                        .foregroundColor(SF.Colors.error)
                        .frame(width: 28, height: 28)
                        .background(SF.Colors.error.opacity(0.12))
                        .cornerRadius(6)
                }
                .buttonStyle(.plain)
            } else if isPaused {
                // Resume
                Button(action: { ProjectStore.shared.setStatus(project.id, status: .active) }) {
                    Image(systemName: "play.fill")
                        .font(.system(size: 12))
                        .foregroundColor(SF.Colors.success)
                        .frame(width: 28, height: 28)
                        .background(SF.Colors.success.opacity(0.12))
                        .cornerRadius(6)
                }
                .buttonStyle(.plain)
                // Stop
                Button(action: { ProjectStore.shared.setStatus(project.id, status: .idea) }) {
                    Image(systemName: "stop.fill")
                        .font(.system(size: 12))
                        .foregroundColor(SF.Colors.error)
                        .frame(width: 28, height: 28)
                        .background(SF.Colors.error.opacity(0.12))
                        .cornerRadius(6)
                }
                .buttonStyle(.plain)
            } else if !isDone {
                // Play
                Button(action: { launchProject() }) {
                    Image(systemName: "play.fill")
                        .font(.system(size: 12))
                        .foregroundColor(SF.Colors.success)
                        .frame(width: 28, height: 28)
                        .background(SF.Colors.success.opacity(0.12))
                        .cornerRadius(6)
                }
                .buttonStyle(.plain)
            }
        }
    }

    private func launchProject() {
        ProjectStore.shared.setStatus(project.id, status: .active)
        Task {
            await bridge.syncLLMConfigAsync()
            bridge.startMissionAsync(projectId: project.id, brief: project.description)
        }
    }
}

// MARK: - Mini Phase Timeline (14 dots, horizontally scrollable, bigger)

struct MiniPhaseTimeline: View {
    let activePhase: Int
    let projectDone: Bool

    private let dotSize: CGFloat = 22
    private let labelWidth: CGFloat = 54
    private let connectorWidth: CGFloat = 10

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 0) {
                ForEach(0..<safePhases.count, id: \.self) { i in
                    HStack(spacing: 0) {
                        phaseDot(index: i)
                        if i < safePhases.count - 1 {
                            phaseConnector(index: i)
                        }
                    }
                }
            }
            .padding(.horizontal, 4)
            .padding(.vertical, 4)
        }
        .frame(height: 50)
    }

    private func phaseDot(index: Int) -> some View {
        let isCompleted = index < activePhase
        let isActive = index == activePhase && !projectDone
        let isDone = projectDone

        return VStack(spacing: 3) {
            ZStack {
                Circle()
                    .fill(dotFill(completed: isCompleted || isDone, active: isActive))
                    .frame(width: dotSize, height: dotSize)

                if isCompleted || isDone {
                    Image(systemName: "checkmark")
                        .font(.system(size: 9, weight: .bold))
                        .foregroundColor(.white)
                } else if isActive {
                    Circle()
                        .stroke(SF.Colors.purple.opacity(0.6), lineWidth: 2)
                        .frame(width: dotSize + 5, height: dotSize + 5)
                    Text("\(index + 1)")
                        .font(.system(size: 9, weight: .bold))
                        .foregroundColor(.white)
                } else {
                    Text("\(index + 1)")
                        .font(.system(size: 9, weight: .semibold))
                        .foregroundColor(SF.Colors.textMuted)
                }
            }

            Text(safePhases[index].short)
                .font(.system(size: 8, weight: .medium))
                .foregroundColor(
                    (isCompleted || isDone) ? SF.Colors.textSecondary :
                    isActive ? SF.Colors.purple :
                    SF.Colors.textMuted.opacity(0.5)
                )
                .lineLimit(1)
                .frame(width: labelWidth)
        }
    }

    private func phaseConnector(index: Int) -> some View {
        let isCompleted = index < activePhase || projectDone
        return Rectangle()
            .fill(isCompleted ? SF.Colors.success.opacity(0.5) : SF.Colors.border.opacity(0.4))
            .frame(width: connectorWidth, height: 2)
            .padding(.bottom, 16)
    }

    private func dotFill(completed: Bool, active: Bool) -> Color {
        if completed { return SF.Colors.success }
        if active    { return SF.Colors.purple }
        return SF.Colors.bgTertiary
    }
}

// MARK: - Color hex-string init (used by ProjectStatus.color)

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
