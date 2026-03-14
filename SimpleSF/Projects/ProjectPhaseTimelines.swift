import SwiftUI

// Ref: FT-SSF-003

// MARK: - Clickable Phase Timeline (expanded card — dots select phase)

struct ClickablePhaseTimeline: View {
    let activePhase: Int
    let projectDone: Bool
    @Binding var selectedIndex: Int?
    let phases: [SFBridge.PhaseInfo]

    private let dotSize: CGFloat = 26
    private let labelWidth: CGFloat = 58
    private let connectorWidth: CGFloat = 10

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 0) {
                ForEach(0..<phases.count, id: \.self) { i in
                    HStack(spacing: 0) {
                        phaseDot(index: i, phase: phases[i])
                        if i < phases.count - 1 {
                            phaseConnector(index: i)
                        }
                    }
                }
            }
            .padding(.horizontal, 4)
            .padding(.vertical, 4)
        }
        .frame(height: 58)
    }

    private func phaseDot(index: Int, phase: SFBridge.PhaseInfo) -> some View {
        let isCompleted = phase.status == "completed"
        let isRunning = phase.status == "running"
        let isFailed = phase.status == "failed" || phase.status == "vetoed"
        let isSelected = selectedIndex == index
        let phaseType = phase.phase_type ?? "once"
        let iteration = phase.iteration ?? 1
        let maxIter = phase.max_iterations ?? 1

        return Button(action: {
            withAnimation(.easeInOut(duration: 0.15)) {
                selectedIndex = selectedIndex == index ? nil : index
            }
        }) {
            VStack(spacing: 3) {
                ZStack {
                    Circle()
                        .fill(dotFill(completed: isCompleted || projectDone, active: isRunning, failed: isFailed))
                        .frame(width: dotSize, height: dotSize)

                    if isSelected {
                        Circle()
                            .stroke(SF.Colors.purple, lineWidth: 2.5)
                            .frame(width: dotSize + 6, height: dotSize + 6)
                    } else if isRunning {
                        Circle()
                            .stroke(SF.Colors.purple.opacity(0.6), lineWidth: 2)
                            .frame(width: dotSize + 5, height: dotSize + 5)
                    }

                    if isCompleted || projectDone {
                        Image(systemName: "checkmark")
                            .font(.system(size: 10, weight: .bold))
                            .foregroundColor(.white)
                    } else if isFailed {
                        Image(systemName: "xmark")
                            .font(.system(size: 10, weight: .bold))
                            .foregroundColor(.white)
                    } else if isRunning {
                        ProgressView().scaleEffect(0.45).tint(.white)
                    } else {
                        Text("\(index + 1)")
                            .font(.system(size: 10, weight: .bold))
                            .foregroundColor(isSelected ? .white : SF.Colors.textMuted)
                    }

                    // Phase type badge (top-right)
                    if phaseType == "sprint" || phaseType == "feedback_loop" {
                        Image(systemName: phaseType == "sprint" ? "arrow.2.squarepath" : "arrow.triangle.2.circlepath")
                            .font(.system(size: 7, weight: .bold))
                            .foregroundColor(.white)
                            .padding(2)
                            .background(SF.Colors.purple.opacity(0.9))
                            .clipShape(Circle())
                            .offset(x: 8, y: -8)
                    } else if phaseType == "gate" {
                        Image(systemName: "lock.shield")
                            .font(.system(size: 7, weight: .bold))
                            .foregroundColor(.white)
                            .padding(2)
                            .background(SF.Colors.warning.opacity(0.9))
                            .clipShape(Circle())
                            .offset(x: 8, y: -8)
                    }
                }

                VStack(spacing: 0) {
                    Text(safePhases[safe: index]?.short ?? phase.phase_name)
                        .font(.system(size: 8, weight: isSelected ? .bold : .medium))
                        .foregroundColor(
                            isSelected ? SF.Colors.purple :
                            (isCompleted || projectDone) ? SF.Colors.textSecondary :
                            isRunning ? SF.Colors.purple :
                            SF.Colors.textMuted.opacity(0.5)
                        )
                        .lineLimit(1)
                        .frame(width: labelWidth)

                    if maxIter > 1 && isRunning {
                        Text("\(iteration)/\(maxIter)")
                            .font(.system(size: 7, weight: .semibold))
                            .foregroundColor(SF.Colors.purple.opacity(0.8))
                    }
                }
            }
        }
        .buttonStyle(.plain)
    }

    private func phaseConnector(index: Int) -> some View {
        let isCompleted = index < activePhase || projectDone
        return Rectangle()
            .fill(isCompleted ? SF.Colors.success.opacity(0.5) : SF.Colors.border.opacity(0.4))
            .frame(width: connectorWidth, height: 2)
            .padding(.bottom, 16)
    }

    private func dotFill(completed: Bool, active: Bool, failed: Bool) -> Color {
        if completed { return SF.Colors.success }
        if failed    { return SF.Colors.error }
        if active    { return SF.Colors.purple }
        return SF.Colors.bgTertiary
    }
}

// MARK: - Mini Phase Timeline (collapsed card — 14 dots)

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
