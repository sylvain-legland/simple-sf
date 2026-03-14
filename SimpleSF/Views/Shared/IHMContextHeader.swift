import SwiftUI

// Ref: FT-SSF-013
// IHM Context Header — shows traceability context for each screen:
// persona, feature, RBAC role, CRUD ops, and linked user stories.

// MARK: - Data Model

struct IHMContext: Sendable {
    let persona: String
    let personaIcon: String          // SF Symbol name
    let featureId: String            // FT-SSF-XXX
    let featureName: String
    let rbacRole: String
    let crudOps: Set<CRUDOp>
    let storyCount: Int
    let acCount: Int

    enum CRUDOp: String, CaseIterable, Sendable {
        case create = "C"
        case read   = "R"
        case update = "U"
        case delete = "D"

        var color: Color {
            switch self {
            case .create: SF.Colors.success
            case .read:   SF.Colors.info
            case .update: SF.Colors.warning
            case .delete: SF.Colors.error
            }
        }
    }

    /// Parse a comma-separated CRUD string like "C,R,U,D"
    static func parseCRUD(_ raw: String) -> Set<CRUDOp> {
        Set(raw.split(separator: ",").compactMap { CRUDOp(rawValue: $0.trimmingCharacters(in: .whitespaces)) })
    }
}

// MARK: - Predefined contexts (from traceability.db)

extension IHMContext {
    static let jarvis = IHMContext(
        persona: "Developer", personaIcon: "person.fill.checkmark",
        featureId: "FT-SSF-001", featureName: "Jarvis AI Chat",
        rbacRole: "all", crudOps: [.create, .read], storyCount: 3, acCount: 7
    )
    static let projects = IHMContext(
        persona: "Tech Lead", personaIcon: "person.2.fill",
        featureId: "FT-SSF-003", featureName: "Project Management",
        rbacRole: "all", crudOps: [.create, .read, .update, .delete], storyCount: 2, acCount: 4
    )
    static let mission = IHMContext(
        persona: "Developer", personaIcon: "person.fill.checkmark",
        featureId: "FT-SSF-004", featureName: "Mission Orchestration",
        rbacRole: "all", crudOps: [.create, .read], storyCount: 2, acCount: 4
    )
    static let ideation = IHMContext(
        persona: "Product Owner", personaIcon: "lightbulb.fill",
        featureId: "FT-SSF-006", featureName: "Ideation Engine",
        rbacRole: "all", crudOps: [.create, .read], storyCount: 1, acCount: 2
    )
    static let agents = IHMContext(
        persona: "Lead Developer", personaIcon: "person.3.fill",
        featureId: "FT-SSF-010", featureName: "Agent Catalog",
        rbacRole: "all", crudOps: [.read], storyCount: 1, acCount: 2
    )
    static let onboarding = IHMContext(
        persona: "New User", personaIcon: "person.badge.plus",
        featureId: "FT-SSF-007", featureName: "Onboarding & Setup",
        rbacRole: "all", crudOps: [.read], storyCount: 1, acCount: 2
    )
    static let setupWizard = IHMContext(
        persona: "New User", personaIcon: "person.badge.plus",
        featureId: "FT-SSF-005", featureName: "LLM Provider Management",
        rbacRole: "all", crudOps: [.create, .read, .update], storyCount: 2, acCount: 4
    )
    static let navigation = IHMContext(
        persona: "All Users", personaIcon: "person.crop.circle",
        featureId: "FT-SSF-001", featureName: "Navigation",
        rbacRole: "all", crudOps: [.read], storyCount: 3, acCount: 7
    )
}

// MARK: - Header View

@MainActor
struct IHMContextHeader: View {
    let context: IHMContext
    @State private var isExpanded = false

    var body: some View {
        VStack(spacing: 0) {
            compactBar
            if isExpanded {
                expandedPanel
                    .transition(.move(edge: .top).combined(with: .opacity))
            }
        }
        .animation(.easeInOut(duration: 0.2), value: isExpanded)
    }

    // MARK: - Compact Bar (~28px)

    private var compactBar: some View {
        HStack(spacing: SF.Spacing.sm) {
            // Persona
            HStack(spacing: 3) {
                Image(systemName: context.personaIcon)
                    .font(.system(size: 9))
                Text(context.persona)
                    .font(SF.Font.badge)
            }
            .foregroundColor(SF.Colors.textSecondary)

            separator

            // Feature badge
            Text(context.featureId)
                .font(SF.Font.badge)
                .foregroundColor(SF.Colors.purple)
                .padding(.horizontal, 5)
                .padding(.vertical, 1)
                .background(SF.Colors.purple.opacity(0.1))
                .cornerRadius(SF.Radius.sm)

            Text(context.featureName)
                .font(SF.Font.badge)
                .foregroundColor(SF.Colors.textMuted)

            separator

            // RBAC
            Text(context.rbacRole.uppercased())
                .font(.system(size: 9, weight: .medium))
                .foregroundColor(SF.Colors.blue)

            separator

            // CRUD indicators
            crudIndicators

            separator

            // Story count
            HStack(spacing: 2) {
                Image(systemName: "doc.text")
                    .font(.system(size: 8))
                Text("\(context.storyCount) US")
                    .font(SF.Font.badge)
            }
            .foregroundColor(SF.Colors.textMuted)

            Spacer()

            // Expand chevron
            Image(systemName: isExpanded ? "chevron.up" : "chevron.down")
                .font(.system(size: 8, weight: .semibold))
                .foregroundColor(SF.Colors.textMuted)
        }
        .padding(.horizontal, SF.Spacing.md)
        .padding(.vertical, SF.Spacing.xs)
        .frame(height: 28)
        .background(SF.Colors.bgSecondary.opacity(0.6))
        .contentShape(Rectangle())
        .onTapGesture { isExpanded.toggle() }
    }

    // MARK: - CRUD Dots

    private var crudIndicators: some View {
        HStack(spacing: 2) {
            ForEach(IHMContext.CRUDOp.allCases, id: \.rawValue) { op in
                Text(op.rawValue)
                    .font(.system(size: 9, weight: .bold, design: .monospaced))
                    .foregroundColor(context.crudOps.contains(op) ? op.color : SF.Colors.textMuted.opacity(0.3))
            }
        }
    }

    // MARK: - Separator

    private var separator: some View {
        Text("·")
            .font(.system(size: 9))
            .foregroundColor(SF.Colors.textMuted.opacity(0.4))
    }

    // MARK: - Expanded Panel

    private var expandedPanel: some View {
        VStack(alignment: .leading, spacing: SF.Spacing.sm) {
            Divider().background(SF.Colors.border.opacity(0.3))

            HStack(alignment: .top, spacing: SF.Spacing.lg) {
                // Left: feature info
                VStack(alignment: .leading, spacing: SF.Spacing.xs) {
                    Label {
                        Text("Feature: \(context.featureId) — \(context.featureName)")
                            .font(SF.Font.caption)
                    } icon: {
                        Image(systemName: "tag.fill")
                            .font(.system(size: 9))
                            .foregroundColor(SF.Colors.purple)
                    }
                    .foregroundColor(SF.Colors.textSecondary)

                    Label {
                        Text("Persona: \(context.persona)")
                            .font(SF.Font.caption)
                    } icon: {
                        Image(systemName: context.personaIcon)
                            .font(.system(size: 9))
                            .foregroundColor(SF.Colors.blue)
                    }
                    .foregroundColor(SF.Colors.textSecondary)

                    Label {
                        Text("RBAC: \(context.rbacRole)")
                            .font(SF.Font.caption)
                    } icon: {
                        Image(systemName: "lock.shield")
                            .font(.system(size: 9))
                            .foregroundColor(SF.Colors.warning)
                    }
                    .foregroundColor(SF.Colors.textSecondary)
                }

                Divider().frame(height: 40)

                // Right: stats
                VStack(alignment: .leading, spacing: SF.Spacing.xs) {
                    Label {
                        Text("CRUD: \(context.crudOps.map(\.rawValue).sorted().joined(separator: ", "))")
                            .font(SF.Font.caption)
                    } icon: {
                        Image(systemName: "pencil.and.list.clipboard")
                            .font(.system(size: 9))
                            .foregroundColor(SF.Colors.success)
                    }
                    .foregroundColor(SF.Colors.textSecondary)

                    Label {
                        Text("\(context.storyCount) User Stories · \(context.acCount) Acceptance Criteria")
                            .font(SF.Font.caption)
                    } icon: {
                        Image(systemName: "checkmark.circle")
                            .font(.system(size: 9))
                            .foregroundColor(SF.Colors.info)
                    }
                    .foregroundColor(SF.Colors.textSecondary)
                }
            }
            .padding(.horizontal, SF.Spacing.md)
            .padding(.bottom, SF.Spacing.sm)
        }
        .background(SF.Colors.bgSecondary.opacity(0.4))
    }
}
