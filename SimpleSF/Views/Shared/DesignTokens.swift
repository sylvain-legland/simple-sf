import SwiftUI
import AppKit

// MARK: - SF Design Tokens (adaptive light/dark theme)

enum SF {
    // MARK: Colors — Adaptive (auto light/dark based on system appearance)
    enum Colors {
        // Backgrounds
        static let bgPrimary    = adaptive(dark: 0x0f0a1a, light: 0xf5f3f8)
        static let bgSecondary  = adaptive(dark: 0x1a1225, light: 0xeae6f0)
        static let bgTertiary   = adaptive(dark: 0x251d33, light: 0xddd8e6)
        static let bgCard       = adaptive(dark: 0x1e1530, light: 0xffffff)
        static let bgHover      = adaptive(dark: 0x302540, light: 0xd5cfe0)

        // Brand purple (stays vibrant in both modes)
        static let purple       = adaptive(dark: 0xbc8cff, light: 0x7c3aed)
        static let purpleLight  = adaptive(dark: 0xc084fc, light: 0x8b5cf6)
        static let purpleDeep   = adaptive(dark: 0x7c3aed, light: 0x6d28d9)
        static let accent       = adaptive(dark: 0xf78166, light: 0xe5603e)
        static let accentHover  = adaptive(dark: 0xffa28b, light: 0xd14f2e)

        // Extended palette
        static let blue         = adaptive(dark: 0x7c8aff, light: 0x4f5bd5)
        static let blueBright   = adaptive(dark: 0x3b82f6, light: 0x2563eb)
        static let blueLight    = adaptive(dark: 0x60a5fa, light: 0x3b82f6)
        static let pink         = adaptive(dark: 0xf472b6, light: 0xdb2777)
        static let cyan         = adaptive(dark: 0x06b6d4, light: 0x0891b2)
        static let teal         = adaptive(dark: 0x14b8a6, light: 0x0d9488)
        static let greenLight   = adaptive(dark: 0x34d399, light: 0x059669)
        static let greenDeep    = adaptive(dark: 0x16a34a, light: 0x15803d)
        static let yellowLight  = adaptive(dark: 0xfbbf24, light: 0xd97706)
        static let yellowDeep   = adaptive(dark: 0xf59e0b, light: 0xb45309)
        static let redLight     = adaptive(dark: 0xf87171, light: 0xdc2626)
        static let redDeep      = adaptive(dark: 0xdc2626, light: 0xb91c1c)

        // Text
        static let textPrimary  = adaptive(dark: 0xe6edf3, light: 0x1a1225)
        static let textSecondary = adaptive(dark: 0x9e95b0, light: 0x57516a)
        static let textMuted    = adaptive(dark: 0x6e7681, light: 0x8b8598)
        static let border       = adaptive(dark: 0x352d45, light: 0xd0c9dd)
        static let borderLight  = adaptive(dark: 0x3d444d, light: 0xc5bfd3)

        // Status (darker in light mode for contrast)
        static let success      = adaptive(dark: 0x22c55e, light: 0x16a34a)
        static let warning      = adaptive(dark: 0xf59e0b, light: 0xd97706)
        static let error        = adaptive(dark: 0xef4444, light: 0xdc2626)
        static let info         = adaptive(dark: 0x6366f1, light: 0x4f46e5)

        // Agent role colors (slightly darker in light mode)
        static let rte          = adaptive(dark: 0x3b82f6, light: 0x2563eb)
        static let po           = adaptive(dark: 0x22c55e, light: 0x16a34a)
        static let architect    = adaptive(dark: 0x6366f1, light: 0x4f46e5)
        static let lead         = adaptive(dark: 0xf59e0b, light: 0xd97706)
        static let dev          = adaptive(dark: 0x06b6d4, light: 0x0891b2)
        static let qa           = adaptive(dark: 0xeab308, light: 0xca8a04)
        static let devops       = adaptive(dark: 0x3b82f6, light: 0x2563eb)
        static let security     = adaptive(dark: 0xef4444, light: 0xdc2626)
        static let ux           = adaptive(dark: 0xec4899, light: 0xdb2777)

        /// Create an adaptive Color that auto-switches between dark and light appearances
        private static func adaptive(dark: UInt, light: UInt) -> Color {
            Color(nsColor: NSColor(name: nil, dynamicProvider: { appearance in
                let isDark = appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
                let hex = isDark ? dark : light
                return NSColor(
                    srgbRed: CGFloat((hex >> 16) & 0xFF) / 255,
                    green:   CGFloat((hex >> 8)  & 0xFF) / 255,
                    blue:    CGFloat(hex & 0xFF)         / 255,
                    alpha:   1.0
                )
            }))
        }
    }

    // MARK: Typography
    enum Font {
        static let mono = SwiftUI.Font.custom("JetBrains Mono", size: 13)
            .monospaced()
        static let monoSmall = SwiftUI.Font.custom("JetBrains Mono", size: 11)
            .monospaced()

        static let title    = SwiftUI.Font.system(size: 18, weight: .bold)
        static let headline = SwiftUI.Font.system(size: 14, weight: .semibold)
        static let body     = SwiftUI.Font.system(size: 13, weight: .regular)
        static let caption  = SwiftUI.Font.system(size: 11, weight: .regular)
        static let badge    = SwiftUI.Font.system(size: 10, weight: .medium)
    }

    // MARK: Spacing
    enum Spacing {
        static let xs: CGFloat = 4
        static let sm: CGFloat = 6
        static let md: CGFloat = 10
        static let lg: CGFloat = 16
        static let xl: CGFloat = 24
    }

    // MARK: Radius
    enum Radius {
        static let sm: CGFloat = 4
        static let md: CGFloat = 8
        static let lg: CGFloat = 12
        static let xl: CGFloat = 16
        static let full: CGFloat = 999
    }
}

// MARK: - Color from hex

extension Color {
    init(hex: UInt, alpha: Double = 1.0) {
        self.init(
            .sRGB,
            red:   Double((hex >> 16) & 0xFF) / 255,
            green: Double((hex >> 8)  & 0xFF) / 255,
            blue:  Double(hex & 0xFF)         / 255,
            opacity: alpha
        )
    }
}

// MARK: - Reusable Components

struct AgentAvatarView: View {
    let agentId: String
    let size: CGFloat

    var body: some View {
        if let nsImage = Self.loadAvatar(agentId) {
            Image(nsImage: nsImage)
                .resizable()
                .aspectRatio(contentMode: .fill)
                .frame(width: size, height: size)
                .clipShape(Circle())
                .overlay(Circle().stroke(SF.Colors.border, lineWidth: 1))
        } else {
            // Fallback: initials circle
            let initials = agentId.split(separator: "_").map { $0.prefix(1).uppercased() }.prefix(2).joined()
            ZStack {
                Circle()
                    .fill(SF.Colors.purpleDeep.opacity(0.3))
                    .overlay(Circle().stroke(SF.Colors.purple.opacity(0.5), lineWidth: 1))
                Text(initials.isEmpty ? "?" : initials)
                    .font(.system(size: size * 0.35, weight: .bold))
                    .foregroundColor(SF.Colors.purpleLight)
            }
            .frame(width: size, height: size)
        }
    }

    private static var cache: [String: NSImage] = [:]
    private static var spmBundle: Bundle? = {
        let bundleName = "SimpleSF_SimpleSF"
        let candidates: [URL?] = [
            Bundle.main.resourceURL,
            Bundle.main.bundleURL,
            Bundle.main.bundleURL.appendingPathComponent("Contents/Resources"),
            // SPM debug: binary sits next to .bundle
            Bundle.main.executableURL?.deletingLastPathComponent(),
        ]
        for candidate in candidates {
            if let path = candidate?.appendingPathComponent(bundleName + ".bundle"),
               let bundle = Bundle(url: path) {
                return bundle
            }
        }
        return nil
    }()

    static func loadAvatar(_ agentId: String) -> NSImage? {
        if let cached = cache[agentId] { return cached }

        // Try SPM module bundle first
        if let bundle = spmBundle {
            for subdir in ["Avatars", "Resources/Avatars", nil] as [String?] {
                if let url = bundle.url(forResource: agentId, withExtension: "jpg", subdirectory: subdir),
                   let img = NSImage(contentsOf: url) {
                    cache[agentId] = img
                    return img
                }
            }
        }

        // Try main bundle
        for subdir in ["Resources/Avatars", "Avatars", nil] as [String?] {
            if let url = Bundle.main.url(forResource: agentId, withExtension: "jpg", subdirectory: subdir),
               let img = NSImage(contentsOf: url) {
                cache[agentId] = img
                return img
            }
        }

        // Direct file path fallback — scan near the executable
        if let execURL = Bundle.main.executableURL {
            let searchRoots = [
                execURL.deletingLastPathComponent().appendingPathComponent("SimpleSF_SimpleSF.bundle/Avatars"),
                execURL.deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("Resources/SimpleSF_SimpleSF.bundle/Avatars"),
            ]
            for root in searchRoots {
                let fileURL = root.appendingPathComponent("\(agentId).jpg")
                if let img = NSImage(contentsOf: fileURL) {
                    cache[agentId] = img
                    return img
                }
            }
        }

        return nil
    }
}

struct RoleBadge: View {
    let role: String
    let color: Color

    var body: some View {
        Text(role.uppercased())
            .font(SF.Font.badge)
            .foregroundColor(color)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(color.opacity(0.15))
            .cornerRadius(SF.Radius.sm)
            .overlay(
                RoundedRectangle(cornerRadius: SF.Radius.sm)
                    .stroke(color.opacity(0.3), lineWidth: 0.5)
            )
    }
}

struct PatternBadge: View {
    let pattern: String

    private var icon: String {
        switch pattern {
        case "network":      return "network"
        case "sequential":   return "arrow.right"
        case "parallel":     return "arrow.triangle.branch"
        case "hierarchical": return "arrow.up.arrow.down"
        case "loop":         return "arrow.trianglehead.2.counterclockwise"
        default:             return "circle.grid.3x3"
        }
    }

    var body: some View {
        HStack(spacing: 3) {
            Image(systemName: icon)
            Text(pattern)
        }
        .font(SF.Font.badge)
        .foregroundColor(SF.Colors.textMuted)
        .padding(.horizontal, 5)
        .padding(.vertical, 2)
        .background(SF.Colors.bgTertiary)
        .cornerRadius(SF.Radius.sm)
    }
}

struct StatusDot: View {
    let active: Bool
    var size: CGFloat = 7

    var body: some View {
        Circle()
            .fill(active ? SF.Colors.success : SF.Colors.textMuted)
            .frame(width: size, height: size)
    }
}

/// Pulsing opacity animation for typing indicators
struct PulseAnimation: ViewModifier {
    var delay: Double = 0
    @State private var animating = false

    func body(content: Content) -> some View {
        content
            .opacity(animating ? 1.0 : 0.2)
            .animation(.easeInOut(duration: 0.6).repeatForever(autoreverses: true).delay(delay), value: animating)
            .onAppear { animating = true }
    }
}
