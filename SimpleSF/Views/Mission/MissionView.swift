import SwiftUI

// MARK: - Value Stream View (SF Legacy "Value Stream - Epics")
// Horizontal phase timeline per epic, click-to-drill agent discussions.
// Phases from product-lifecycle workflow: 14 phases, each with pattern + gate.

// Ref: FT-SSF-004
struct MissionView: View {
    @ObservedObject var bridge = SFBridge.shared
    @ObservedObject var catalog = SFCatalog.shared
    @State private var selectedProject: SFBridge.SFProject?
    @State private var brief = ""
    @State var status: SFBridge.MissionStatus?
    @State var selectedPhaseIndex: Int?
    @State private var pollTimer: Timer?
    @State private var missionLoadingState: LoadingState = .loading  // Ref: FT-SSF-013

    var body: some View {
        VStack(spacing: 0) {
            IHMContextHeader(context: .mission)

            if missionLoadingState == .loading {
                // Ref: FT-SSF-013 — Skeleton while determining mission state
                SkeletonMissionView()
                    .padding(SF.Spacing.xl)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if !bridge.isRunning && bridge.currentMissionId == nil {
                launchForm
            } else {
                valueStreamView
            }
        }
        .background(SF.Colors.bgPrimary)
        .onAppear {
            startPolling()
            // Brief skeleton then show real content
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
                missionLoadingState = .loaded
            }
        }
        .onDisappear { stopPolling() }
    }

    // MARK: - Launch Form

    private var launchForm: some View {
        VStack(spacing: 0) {
            // Header
            HStack(spacing: 10) {
                Image(systemName: "flowchart.fill")
                    .font(.system(size: 20))
                    .foregroundColor(SF.Colors.purple)
                Text("Value Stream")
                    .font(.system(size: 22, weight: .bold))
                    .foregroundColor(SF.Colors.textPrimary)
                Spacer()
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 16)

            Divider().background(SF.Colors.border)

            Spacer()

            VStack(spacing: 24) {
                Image(systemName: "rocket.fill")
                    .font(.system(size: 52))
                    .foregroundColor(SF.Colors.purple.opacity(0.5))

                Text("Lancer un Epic")
                    .font(.system(size: 20, weight: .bold))
                    .foregroundColor(SF.Colors.textPrimary)

                Text("Décrivez votre produit. L'équipe SAFe enchaîne 14 phases :\nIdéation → Comité Stratégique → Architecture → Sprints Dev → QA → Deploy Prod → TMA")
                    .font(.system(size: 13))
                    .foregroundColor(SF.Colors.textSecondary)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 600)

                let projects = bridge.listProjects()
                if !projects.isEmpty {
                    Picker("Projet", selection: $selectedProject) {
                        Text("Sélectionner un projet…").tag(nil as SFBridge.SFProject?)
                        ForEach(projects) { p in
                            Text(p.name).tag(p as SFBridge.SFProject?)
                        }
                    }
                    .frame(maxWidth: 400)
                }

                TextEditor(text: $brief)
                    .font(.system(size: 13).monospaced())
                    .foregroundColor(SF.Colors.textPrimary)
                    .scrollContentBackground(.hidden)
                    .frame(maxWidth: 600, minHeight: 100, maxHeight: 120)
                    .padding(12)
                    .background(SF.Colors.bgTertiary)
                    .cornerRadius(10)
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .stroke(SF.Colors.border, lineWidth: 1)
                    )

                Button(action: launchMission) {
                    HStack(spacing: 8) {
                        Image(systemName: "play.fill")
                        Text("Lancer le Workflow SAFe")
                            .font(.system(size: 14, weight: .semibold))
                    }
                    .padding(.horizontal, 24)
                    .padding(.vertical, 10)
                    .background(SF.Colors.purple)
                    .foregroundColor(.white)
                    .cornerRadius(10)
                }
                .buttonStyle(.plain)
                .disabled(brief.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }

            Spacer()
        }
    }

    // MARK: - Value Stream (Epic Timeline + Phase Detail)

    private var valueStreamView: some View {
        VStack(spacing: 0) {
            epicHeader
            Divider().background(SF.Colors.border)
            phaseTimeline
            Divider().background(SF.Colors.border)
            phaseDetailOrFeed
        }
    }

    // ── Epic header banner ──

    private var epicHeader: some View {
        HStack(spacing: 14) {
            Image(systemName: "flowchart.fill")
                .font(.system(size: 18))
                .foregroundColor(SF.Colors.purple)

            VStack(alignment: .leading, spacing: 2) {
                Text("Cycle de Vie Produit Complet")
                    .font(.system(size: 16, weight: .bold))
                    .foregroundColor(SF.Colors.textPrimary)
                Text(status?.mission?.brief.prefix(120) ?? brief.prefix(120))
                    .font(.system(size: 12))
                    .foregroundColor(SF.Colors.textSecondary)
                    .lineLimit(1)
            }

            Spacer()

            missionStatusBadge
        }
        .padding(.horizontal, 24)
        .padding(.vertical, 14)
        .background(SF.Colors.bgSecondary)
    }

    @ViewBuilder
    private var missionStatusBadge: some View {
        let s = status?.mission?.status ?? "running"
        let (label, color, icon): (String, Color, String) = {
            switch s {
            case "completed": return ("Terminé", SF.Colors.success, "checkmark.circle.fill")
            case "failed":    return ("Échoué", SF.Colors.error, "xmark.circle.fill")
            case "vetoed":    return ("Véto", SF.Colors.warning, "exclamationmark.triangle.fill")
            default:          return ("En cours", SF.Colors.purple, "play.circle.fill")
            }
        }()
        HStack(spacing: 6) {
            if s == "running" { ProgressView().scaleEffect(0.6) }
            Image(systemName: icon).font(.system(size: 12))
            Text(label).font(.system(size: 12, weight: .semibold))
        }
        .foregroundColor(color)
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .background(color.opacity(0.12))
        .cornerRadius(8)
    }

    // ── Horizontal phase timeline ── (see MissionPhaseView.swift)

    // ── Phase detail + agent feed ── (see MissionPhaseView.swift)

    // ── Live events feed ── (see MissionAgentPanel.swift)

    // MARK: - Launch & Polling

    private func launchMission() {
        let projectId = selectedProject?.id ?? "default"
        let _ = bridge.startMission(projectId: projectId, brief: brief)
        brief = ""
    }

    private func startPolling() {
        pollTimer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { _ in
            Task { @MainActor in
                self.status = bridge.missionStatus()
            }
        }
    }

    private func stopPolling() {
        pollTimer?.invalidate()
        pollTimer = nil
    }
}
