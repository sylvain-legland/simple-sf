import SwiftUI

// Ref: FT-SSF-013, FT-SSF-015
// MARK: - Shimmer Animation

/// A linear gradient sweep that animates left-to-right over 1.5s, looping infinitely.
struct ShimmerModifier: ViewModifier {
    @State private var phase: CGFloat = -1

    func body(content: Content) -> some View {
        content
            .overlay(
                GeometryReader { geo in
                    LinearGradient(
                        gradient: Gradient(stops: [
                            .init(color: .clear, location: max(0, phase - 0.3)),
                            .init(color: SF.Colors.bgTertiary.opacity(0.6), location: phase),
                            .init(color: .clear, location: min(1, phase + 0.3)),
                        ]),
                        startPoint: .leading,
                        endPoint: .trailing
                    )
                    .blendMode(.sourceAtop)
                }
            )
            .onAppear {
                withAnimation(
                    .linear(duration: 1.5)
                    .repeatForever(autoreverses: false)
                ) {
                    phase = 2
                }
            }
    }
}

extension View {
    func shimmer() -> some View {
        modifier(ShimmerModifier())
    }
}

// MARK: - Skeleton Primitives

/// A rounded rectangle placeholder line with shimmer animation.
struct SkeletonLine: View {
    var width: CGFloat? = nil
    var height: CGFloat = 14

    var body: some View {
        RoundedRectangle(cornerRadius: SF.Radius.sm)
            .fill(SF.Colors.bgSecondary)
            .frame(width: width, height: height)
            .shimmer()
            .accessibilityLabel("Loading…")
    }
}

/// A circular placeholder (avatars, status dots).
struct SkeletonCircle: View {
    var size: CGFloat = 40

    var body: some View {
        Circle()
            .fill(SF.Colors.bgSecondary)
            .frame(width: size, height: size)
            .shimmer()
            .accessibilityLabel("Loading…")
    }
}

/// A card-shaped skeleton matching the project/agent card layout.
struct SkeletonCard: View {
    var body: some View {
        VStack(alignment: .leading, spacing: SF.Spacing.md) {
            HStack(spacing: SF.Spacing.md) {
                SkeletonCircle(size: 36)
                VStack(alignment: .leading, spacing: SF.Spacing.xs) {
                    SkeletonLine(width: 120, height: 14)
                    SkeletonLine(width: 80, height: 10)
                }
            }
            SkeletonLine(height: 12)
            SkeletonLine(width: 200, height: 12)
        }
        .padding(SF.Spacing.lg)
        .background(SF.Colors.bgSecondary.opacity(0.3))
        .cornerRadius(SF.Radius.lg)
        .overlay(
            RoundedRectangle(cornerRadius: SF.Radius.lg)
                .stroke(SF.Colors.border.opacity(0.2), lineWidth: 1)
        )
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Loading…")
    }
}

/// Multiple skeleton lines simulating a list of items.
struct SkeletonList: View {
    var rows: Int = 5

    var body: some View {
        VStack(alignment: .leading, spacing: SF.Spacing.md) {
            ForEach(0..<rows, id: \.self) { index in
                HStack(spacing: SF.Spacing.md) {
                    SkeletonCircle(size: 32)
                    VStack(alignment: .leading, spacing: SF.Spacing.xs) {
                        SkeletonLine(width: randomWidth(for: index), height: 13)
                        SkeletonLine(width: randomWidth(for: index + 10) * 0.7, height: 10)
                    }
                }
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Loading…")
    }

    private func randomWidth(for seed: Int) -> CGFloat {
        let widths: [CGFloat] = [180, 220, 160, 200, 140]
        return widths[seed % widths.count]
    }
}

/// A small badge-shaped skeleton placeholder.
struct SkeletonBadge: View {
    var body: some View {
        RoundedRectangle(cornerRadius: SF.Radius.sm)
            .fill(SF.Colors.bgSecondary)
            .frame(width: 60, height: 18)
            .shimmer()
            .accessibilityLabel("Loading…")
    }
}

// MARK: - Contextual Skeletons

/// Skeleton matching the AgentsView grid card layout.
struct SkeletonAgentGrid: View {
    var count: Int = 6

    var body: some View {
        LazyVGrid(
            columns: [GridItem(.adaptive(minimum: 250), spacing: SF.Spacing.lg)],
            spacing: SF.Spacing.lg
        ) {
            ForEach(0..<count, id: \.self) { _ in
                SkeletonCard()
            }
        }
        .padding()
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Loading agents…")
    }
}

/// Skeleton matching the ProjectsView accordion list layout.
struct SkeletonProjectList: View {
    var count: Int = 4

    var body: some View {
        VStack(spacing: SF.Spacing.lg) {
            ForEach(0..<count, id: \.self) { _ in
                VStack(alignment: .leading, spacing: SF.Spacing.md) {
                    HStack(spacing: SF.Spacing.md) {
                        SkeletonLine(width: 16, height: 16)
                        SkeletonLine(width: 180, height: 16)
                        Spacer()
                        SkeletonBadge()
                        SkeletonCircle(size: 8)
                    }
                    SkeletonLine(height: 12)
                    HStack(spacing: SF.Spacing.sm) {
                        SkeletonLine(width: 100, height: 6)
                        Spacer()
                        SkeletonLine(width: 70, height: 10)
                    }
                }
                .padding(SF.Spacing.lg)
                .background(SF.Colors.bgSecondary.opacity(0.3))
                .cornerRadius(SF.Radius.lg)
                .overlay(
                    RoundedRectangle(cornerRadius: SF.Radius.lg)
                        .stroke(SF.Colors.border.opacity(0.2), lineWidth: 1)
                )
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Loading projects…")
    }
}

/// Skeleton matching the Jarvis chat area with message bubbles.
struct SkeletonChatView: View {
    var body: some View {
        VStack(alignment: .leading, spacing: SF.Spacing.xl) {
            // Simulated agent message
            HStack(alignment: .top, spacing: SF.Spacing.md) {
                SkeletonCircle(size: 32)
                VStack(alignment: .leading, spacing: SF.Spacing.sm) {
                    HStack(spacing: SF.Spacing.sm) {
                        SkeletonLine(width: 100, height: 13)
                        SkeletonBadge()
                    }
                    SkeletonLine(height: 12)
                    SkeletonLine(width: 240, height: 12)
                    SkeletonLine(width: 160, height: 12)
                }
            }

            // Simulated user message (right-aligned)
            HStack {
                Spacer()
                SkeletonLine(width: 200, height: 36)
            }

            // Another agent message
            HStack(alignment: .top, spacing: SF.Spacing.md) {
                SkeletonCircle(size: 32)
                VStack(alignment: .leading, spacing: SF.Spacing.sm) {
                    SkeletonLine(width: 120, height: 13)
                    SkeletonLine(height: 12)
                    SkeletonLine(width: 180, height: 12)
                }
            }
        }
        .padding(SF.Spacing.xl)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Loading chat…")
    }
}

/// Skeleton matching the MissionView phase timeline.
struct SkeletonMissionView: View {
    var body: some View {
        VStack(alignment: .leading, spacing: SF.Spacing.xl) {
            // Epic header skeleton
            HStack(spacing: SF.Spacing.md) {
                SkeletonLine(width: 20, height: 20)
                SkeletonLine(width: 240, height: 18)
                Spacer()
                SkeletonBadge()
            }

            // Phase timeline skeleton — horizontal row of circles
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: SF.Spacing.lg) {
                    ForEach(0..<14, id: \.self) { _ in
                        VStack(spacing: SF.Spacing.xs) {
                            SkeletonCircle(size: 28)
                            SkeletonLine(width: 50, height: 9)
                        }
                    }
                }
                .padding(.horizontal, SF.Spacing.xl)
            }
            .frame(height: 60)

            // Event feed skeleton
            SkeletonList(rows: 4)
                .padding(.horizontal, SF.Spacing.xl)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Loading mission…")
    }
}
