import SwiftUI

// Ref: FT-SSF-007
// Local LLM provider cards (Ollama + MLX) for the Settings view.

extension OnboardingView {

    // MARK: - Local LLM Section

    var localLLMSection: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Image(systemName: "desktopcomputer")
                    .foregroundColor(SF.Colors.purple)
                Text(L10n.shared.t(.settingsLocalModels))
                    .font(.headline)
                    .foregroundColor(SF.Colors.textPrimary)
                Spacer()
                Text(L10n.shared.t(.settingsLocalHint))
                    .font(.caption2)
                    .foregroundColor(SF.Colors.textSecondary)
            }

            ollamaCard
            mlxCard
        }
        .padding()
        .background(SF.Colors.bgSecondary)
        .cornerRadius(12)
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(
                    (ollama.isRunning || mlx.isRunning) ? Color.green.opacity(0.4) : SF.Colors.border,
                    lineWidth: 1.5
                )
        )
    }

    // MARK: - Ollama Card

    private var ollamaCard: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Ollama")
                        .font(.subheadline.bold())
                        .foregroundColor(SF.Colors.textPrimary)
                    Text(LLMProvider.ollama.subtitle)
                        .font(.caption2)
                        .foregroundColor(SF.Colors.textMuted)
                }
                if ollama.isRunning {
                    Label(L10n.shared.t(.statusRunning), systemImage: "circle.fill")
                        .font(.caption2)
                        .foregroundColor(.green)
                } else {
                    Label(L10n.shared.t(.statusStopped), systemImage: "circle")
                        .font(.caption2)
                        .foregroundColor(SF.Colors.textMuted)
                }
                Spacer()
                if ollama.isRunning {
                    useButton(.ollama, available: true)
                }
                Button(action: { Task { await ollama.refresh() } }) {
                    Image(systemName: "arrow.clockwise")
                        .font(.caption)
                }
                .buttonStyle(.plain)
                .foregroundColor(SF.Colors.purple)
            }

            if ollama.isRunning {
                if !ollama.availableModels.isEmpty {
                    HStack {
                        Text(L10n.shared.t(.settingsModel))
                            .font(.callout)
                            .foregroundColor(SF.Colors.textSecondary)
                        Picker("", selection: $ollama.activeModel) {
                            ForEach(ollama.availableModels) { model in
                                HStack {
                                    Text(model.name)
                                    Text("(\(model.size))")
                                        .foregroundColor(.secondary)
                                }
                                .tag(model as OllamaService.OllamaModel?)
                            }
                        }
                        .labelsHidden()
                        .frame(maxWidth: 300)
                    }
                } else {
                    Text(L10n.shared.t(.setupNoOllamaModels))
                        .font(.caption)
                        .foregroundColor(SF.Colors.textSecondary)
                }

                HStack(spacing: 12) {
                    Button(action: { ollama.stop() }) {
                        Label(L10n.shared.t(.actionStop), systemImage: "stop.circle.fill")
                    }
                    .buttonStyle(.bordered)
                    .tint(.red)
                    .controlSize(.small)

                    Text("Port \(ollama.port)")
                        .font(.caption.monospaced())
                        .foregroundColor(SF.Colors.textSecondary)
                }
            } else {
                HStack(spacing: 12) {
                    Button(action: { ollama.start() }) {
                        Label(L10n.shared.t(.setupStartOllama), systemImage: "play.circle.fill")
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(SF.Colors.purple)
                    .controlSize(.small)

                    Text("or run: ollama serve")
                        .font(.caption2)
                        .foregroundColor(SF.Colors.textSecondary)
                }
            }
        }
        .padding(12)
        .background(cardBg(.ollama))
        .cornerRadius(8)
        .overlay(selectionBorder(.ollama))
    }

    // MARK: - MLX Card

    private var mlxCard: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text(L10n.shared.t(.setupAppleMLX))
                        .font(.subheadline.bold())
                        .foregroundColor(SF.Colors.textPrimary)
                    Text(LLMProvider.mlx.subtitle)
                        .font(.caption2)
                        .foregroundColor(SF.Colors.textMuted)
                }
                mlxStatusBadge
                Spacer()
                if mlx.isRunning {
                    useButton(.mlx, available: true)
                }
                Button(action: { mlx.scanModels() }) {
                    Image(systemName: "arrow.clockwise")
                        .font(.caption)
                }
                .buttonStyle(.plain)
                .foregroundColor(SF.Colors.purple)
            }

            if !mlx.availableModels.isEmpty {
                HStack {
                    Text(L10n.shared.t(.settingsModel))
                        .font(.callout)
                        .foregroundColor(SF.Colors.textSecondary)
                    Picker("", selection: $mlx.activeModel) {
                        ForEach(mlx.availableModels) { model in
                            HStack {
                                Text(model.name)
                                if !model.modelType.isEmpty {
                                    Text(model.modelType)
                                        .foregroundColor(.secondary)
                                }
                                Text(String(format: "%.1f GB", model.sizeGB))
                                    .foregroundColor(.secondary)
                            }
                            .tag(model as MLXService.MLXModel?)
                        }
                    }
                    .labelsHidden()
                    .frame(maxWidth: 300)
                }
            } else {
                Text(L10n.shared.t(.setupNoMLXModels))
                    .font(.caption)
                    .foregroundColor(SF.Colors.textSecondary)
            }

            HStack(spacing: 12) {
                if mlx.isRunning {
                    Button(action: { mlx.stop() }) {
                        Label(L10n.shared.t(.actionStop), systemImage: "stop.circle.fill")
                    }
                    .buttonStyle(.bordered)
                    .tint(.red)
                    .controlSize(.small)

                    Text("Port \(mlx.port)")
                        .font(.caption.monospaced())
                        .foregroundColor(SF.Colors.textSecondary)
                } else {
                    Button(action: {
                        mlx.start()
                        Task {
                            try? await Task.sleep(nanoseconds: 5_000_000_000)
                            if mlx.isRunning { SFBridge.shared.syncLLMConfig() }
                        }
                    }) {
                        Label(L10n.shared.t(.setupStartServer), systemImage: "play.circle.fill")
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(SF.Colors.purple)
                    .controlSize(.small)
                    .disabled(mlx.activeModel == nil)
                }

                Spacer()

                Toggle(isOn: Binding(
                    get: { AppState.shared.mlxAutoRestart },
                    set: { AppState.shared.setMLXAutoRestart($0) }
                )) {
                    Text(L10n.shared.t(.settingsAutoStart))
                        .font(.caption)
                        .foregroundColor(SF.Colors.textSecondary)
                }
                .toggleStyle(.switch)
                .controlSize(.mini)
            }

            if !mlx.logLines.isEmpty {
                ScrollView(.vertical) {
                    VStack(alignment: .leading, spacing: 2) {
                        ForEach(Array(mlx.logLines.suffix(3).enumerated()), id: \.offset) { _, line in
                            Text(line)
                                .font(.caption2.monospaced())
                                .foregroundColor(SF.Colors.textMuted)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                .frame(maxHeight: 40)
            }
        }
        .padding(12)
        .background(cardBg(.mlx))
        .cornerRadius(8)
        .overlay(selectionBorder(.mlx))
    }

    @ViewBuilder
    private var mlxStatusBadge: some View {
        switch mlx.state {
        case .stopped:
            Label(L10n.shared.t(.statusStopped), systemImage: "circle")
                .font(.caption2)
                .foregroundColor(SF.Colors.textMuted)
        case .starting:
            HStack(spacing: 4) {
                ProgressView().scaleEffect(0.5)
                Text(L10n.shared.t(.statusStarting))
                    .font(.caption2)
                    .foregroundColor(.orange)
            }
        case .running:
            Label(L10n.shared.t(.statusRunning), systemImage: "circle.fill")
                .font(.caption2)
                .foregroundColor(.green)
        case .error(let msg):
            Label(msg, systemImage: "exclamationmark.circle.fill")
                .font(.caption2)
                .foregroundColor(.red)
                .lineLimit(1)
        }
    }
}
