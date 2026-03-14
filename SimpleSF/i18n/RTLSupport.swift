import SwiftUI

// Ref: FT-SSF-015
// RTL layout helpers for right-to-left language support.
// RTL languages: ar (Arabic), he (Hebrew), fa (Farsi), ur (Urdu), ps (Pashto), ku (Kurdish)

// MARK: - RTL-aware view modifier

struct RTLAwareModifier: ViewModifier {
    @ObservedObject private var l10n = L10n.shared

    func body(content: Content) -> some View {
        content
            .environment(\.layoutDirection, l10n.layoutDirection)
    }
}

extension View {
    /// Apply RTL layout direction based on current locale
    func rtlAware() -> some View {
        modifier(RTLAwareModifier())
    }
}

// MARK: - Flippable icon modifier (chevrons, arrows)

struct FlipForRTLModifier: ViewModifier {
    @ObservedObject private var l10n = L10n.shared

    func body(content: Content) -> some View {
        content
            .scaleEffect(x: l10n.isRTL ? -1 : 1, y: 1)
    }
}

extension View {
    /// Flip horizontally for RTL (for directional icons like chevrons/arrows)
    func flipForRTL() -> some View {
        modifier(FlipForRTLModifier())
    }
}

// MARK: - RTL-aware alignment

extension HorizontalAlignment {
    /// Returns .leading in LTR, .trailing in RTL
    @MainActor
    static var rtlLeading: HorizontalAlignment {
        L10n.shared.isRTL ? .trailing : .leading
    }

    /// Returns .trailing in LTR, .leading in RTL
    @MainActor
    static var rtlTrailing: HorizontalAlignment {
        L10n.shared.isRTL ? .leading : .trailing
    }
}

// MARK: - RTL-aware edge insets

extension EdgeInsets {
    /// Create edge insets that respect RTL layout
    @MainActor
    static func rtl(top: CGFloat = 0, leading: CGFloat = 0, bottom: CGFloat = 0, trailing: CGFloat = 0) -> EdgeInsets {
        if L10n.shared.isRTL {
            return EdgeInsets(top: top, leading: trailing, bottom: bottom, trailing: leading)
        }
        return EdgeInsets(top: top, leading: leading, bottom: bottom, trailing: trailing)
    }
}

// MARK: - RTL-aware text alignment

extension TextAlignment {
    /// Natural text alignment (leading edge) respecting RTL
    @MainActor
    static var rtlNatural: TextAlignment {
        L10n.shared.isRTL ? .trailing : .leading
    }
}

// MARK: - RTL environment key (for views that need to read it)

struct RTLEnvironmentKey: EnvironmentKey {
    static let defaultValue: Bool = false
}

extension EnvironmentValues {
    var isRTL: Bool {
        get { self[RTLEnvironmentKey.self] }
        set { self[RTLEnvironmentKey.self] = newValue }
    }
}

// MARK: - RTL-aware NavigationSplitView wrapper

struct RTLNavigationSplitView<Sidebar: View, Detail: View>: View {
    @ObservedObject private var l10n = L10n.shared
    let sidebar: () -> Sidebar
    let detail: () -> Detail

    init(@ViewBuilder sidebar: @escaping () -> Sidebar,
         @ViewBuilder detail: @escaping () -> Detail) {
        self.sidebar = sidebar
        self.detail = detail
    }

    var body: some View {
        NavigationSplitView {
            sidebar()
        } detail: {
            detail()
        }
        .environment(\.layoutDirection, l10n.layoutDirection)
    }
}
