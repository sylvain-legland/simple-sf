import SwiftUI

// Ref: FT-SSF-013
// MARK: - Loading State Machine

/// Represents the state of any async data-loading view.
enum LoadingState: Equatable {
    case loading
    case loaded
    case empty(String)
    case error(String)
    case offline

    static func == (lhs: LoadingState, rhs: LoadingState) -> Bool {
        switch (lhs, rhs) {
        case (.loading, .loading), (.loaded, .loaded), (.offline, .offline):
            return true
        case (.empty(let a), .empty(let b)), (.error(let a), .error(let b)):
            return a == b
        default:
            return false
        }
    }
}

// MARK: - Loading State View

/// A generic wrapper that shows skeleton, content, empty, error, or offline state.
struct LoadingStateView<Content: View, Skeleton: View>: View {
    let state: LoadingState
    let skeleton: () -> Skeleton
    let content: () -> Content
    var onRetry: (() -> Void)?
    var emptyAction: (() -> Void)?
    var emptyActionLabel: String = "Refresh"

    init(
        state: LoadingState,
        @ViewBuilder skeleton: @escaping () -> Skeleton,
        @ViewBuilder content: @escaping () -> Content,
        onRetry: (() -> Void)? = nil,
        emptyAction: (() -> Void)? = nil,
        emptyActionLabel: String = "Refresh"
    ) {
        self.state = state
        self.skeleton = skeleton
        self.content = content
        self.onRetry = onRetry
        self.emptyAction = emptyAction
        self.emptyActionLabel = emptyActionLabel
    }

    var body: some View {
        switch state {
        case .loading:
            skeleton()
                .transition(.opacity)

        case .loaded:
            content()
                .transition(.opacity)

        case .empty(let message):
            emptyStateView(message: message)
                .transition(.opacity)

        case .error(let message):
            errorStateView(message: message)
                .transition(.opacity)

        case .offline:
            offlineStateView
                .transition(.opacity)
        }
    }

    // MARK: - Empty State

    private func emptyStateView(message: String) -> some View {
        VStack(spacing: SF.Spacing.lg) {
            Spacer()

            Image(systemName: "tray")
                .font(.system(size: 40))
                .foregroundColor(SF.Colors.textMuted)

            Text(message)
                .font(SF.Font.body)
                .foregroundColor(SF.Colors.textSecondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 300)

            if let emptyAction {
                Button(action: emptyAction) {
                    HStack(spacing: SF.Spacing.sm) {
                        Image(systemName: "arrow.clockwise")
                        Text(emptyActionLabel)
                    }
                    .font(SF.Font.headline)
                    .foregroundColor(.white)
                    .padding(.horizontal, SF.Spacing.lg)
                    .padding(.vertical, SF.Spacing.md)
                    .background(SF.Colors.purple)
                    .cornerRadius(SF.Radius.md)
                }
                .buttonStyle(.plain)
            }

            Spacer()
        }
        .frame(maxWidth: .infinity)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(message)
    }

    // MARK: - Error State

    private func errorStateView(message: String) -> some View {
        VStack(spacing: SF.Spacing.lg) {
            Spacer()

            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 40))
                .foregroundColor(SF.Colors.warning)

            Text("Something went wrong")
                .font(SF.Font.headline)
                .foregroundColor(SF.Colors.textPrimary)

            Text(message)
                .font(SF.Font.body)
                .foregroundColor(SF.Colors.textSecondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 350)

            if let onRetry {
                Button(action: onRetry) {
                    HStack(spacing: SF.Spacing.sm) {
                        Image(systemName: "arrow.clockwise")
                        Text("Try Again")
                    }
                    .font(SF.Font.headline)
                    .foregroundColor(.white)
                    .padding(.horizontal, SF.Spacing.lg)
                    .padding(.vertical, SF.Spacing.md)
                    .background(SF.Colors.purple)
                    .cornerRadius(SF.Radius.md)
                }
                .buttonStyle(.plain)
            }

            Spacer()
        }
        .frame(maxWidth: .infinity)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Error: \(message)")
    }

    // MARK: - Offline State

    private var offlineStateView: some View {
        VStack(spacing: SF.Spacing.lg) {
            Spacer()

            Image(systemName: "wifi.slash")
                .font(.system(size: 40))
                .foregroundColor(SF.Colors.textMuted)

            Text("You're offline")
                .font(SF.Font.headline)
                .foregroundColor(SF.Colors.textPrimary)

            Text("Changes will sync when you reconnect.")
                .font(SF.Font.body)
                .foregroundColor(SF.Colors.textSecondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 300)

            if let onRetry {
                Button(action: onRetry) {
                    HStack(spacing: SF.Spacing.sm) {
                        Image(systemName: "arrow.clockwise")
                        Text("Retry")
                    }
                    .font(SF.Font.headline)
                    .foregroundColor(.white)
                    .padding(.horizontal, SF.Spacing.lg)
                    .padding(.vertical, SF.Spacing.md)
                    .background(SF.Colors.purple)
                    .cornerRadius(SF.Radius.md)
                }
                .buttonStyle(.plain)
            }

            Spacer()
        }
        .frame(maxWidth: .infinity)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Offline. Changes will sync when you reconnect.")
    }
}
