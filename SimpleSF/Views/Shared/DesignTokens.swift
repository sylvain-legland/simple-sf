import SwiftUI

// MARK: - SF Design Tokens (matching SF legacy platform theme)

enum SF {
    // MARK: Colors — SF Legacy dark theme (purple-tinted backgrounds)
    enum Colors {
        // Backgrounds — purple-tinted (SF legacy palette)
        static let bgPrimary    = Color(hex: 0x0f0a1a)   // deep purple-black
        static let bgSecondary  = Color(hex: 0x1a1225)   // very dark purple
        static let bgTertiary   = Color(hex: 0x251d33)   // dark purple
        static let bgCard       = Color(hex: 0x1e1530)   // elevated cards
        static let bgHover      = Color(hex: 0x302540)   // hover states

        // Brand purple
        static let purple       = Color(hex: 0xbc8cff)   // lavender (SF legacy)
        static let purpleLight  = Color(hex: 0xc084fc)
        static let purpleDeep   = Color(hex: 0x7c3aed)
        static let accent       = Color(hex: 0xf78166)   // coral/orange
        static let accentHover  = Color(hex: 0xffa28b)

        // Extended palette (SF legacy)
        static let blue         = Color(hex: 0x7c8aff)   // periwinkle
        static let blueBright   = Color(hex: 0x3b82f6)
        static let blueLight    = Color(hex: 0x60a5fa)
        static let pink         = Color(hex: 0xf472b6)
        static let cyan         = Color(hex: 0x06b6d4)
        static let teal         = Color(hex: 0x14b8a6)
        static let greenLight   = Color(hex: 0x34d399)   // emerald
        static let greenDeep    = Color(hex: 0x16a34a)
        static let yellowLight  = Color(hex: 0xfbbf24)
        static let yellowDeep   = Color(hex: 0xf59e0b)
        static let redLight     = Color(hex: 0xf87171)
        static let redDeep      = Color(hex: 0xdc2626)

        // Text
        static let textPrimary  = Color(hex: 0xe6edf3)
        static let textSecondary = Color(hex: 0x9e95b0)  // mauve (SF legacy)
        static let textMuted    = Color(hex: 0x6e7681)
        static let border       = Color(hex: 0x352d45)   // muted purple border
        static let borderLight  = Color(hex: 0x3d444d)

        // Status
        static let success      = Color(hex: 0x22c55e)
        static let warning      = Color(hex: 0xf59e0b)
        static let error        = Color(hex: 0xef4444)
        static let info         = Color(hex: 0x6366f1)

        // Agent role colors
        static let rte          = Color(hex: 0x3b82f6) // blue
        static let po           = Color(hex: 0x22c55e) // green
        static let architect    = Color(hex: 0x6366f1) // indigo
        static let lead         = Color(hex: 0xf59e0b) // orange
        static let dev          = Color(hex: 0x06b6d4) // cyan
        static let qa           = Color(hex: 0xeab308) // yellow
        static let devops       = Color(hex: 0x3b82f6) // blue
        static let security     = Color(hex: 0xef4444) // red
        static let ux           = Color(hex: 0xec4899) // pink
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
