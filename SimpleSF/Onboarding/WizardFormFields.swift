import SwiftUI

// Ref: FT-SSF-007
// Model selection and download progress views for the setup wizard.

extension SetupWizardView {

    // MARK: - Step 3a: MLX Model Pick

    var mlxModelStep: some View {
        VStack(spacing: 20) {
            Text("Choose your model")
                .font(.title2.bold())
                .foregroundColor(.white)

            HStack(spacing: 6) {
                Image(systemName: "memorychip")
                    .foregroundColor(.purple)
                Text("\(ramGB) GB unified memory — recommended:")
                    .font(.callout)
                    .foregroundColor(.gray)
                Text(recommended.name)
                    .font(.callout.bold())
                    .foregroundColor(.purple)
            }

            VStack(spacing: 8) {
                ForEach(HuggingFaceService.curatedModels) { model in
                    let fits = model.minRAMGB <= ramGB
                    let isRec = model.repoId == recommended.repoId
                    let isChosen = selectedModel?.repoId == model.repoId
                    let alreadyHave = hf.isDownloaded(model)

                    Button(action: { if fits { selectedModel = model } }) {
                        HStack(spacing: 12) {
                            Image(systemName: isChosen ? "checkmark.circle.fill" : "circle")
                                .foregroundColor(isChosen ? .purple : .gray)

                            VStack(alignment: .leading, spacing: 2) {
                                HStack {
                                    Text(model.name)
                                        .font(.headline)
                                        .foregroundColor(fits ? .white : .gray)
                                    if isRec {
                                        Text("Recommended")
                                            .font(.caption2.bold())
                                            .padding(.horizontal, 6)
                                            .padding(.vertical, 2)
                                            .background(Color.purple)
                                            .foregroundColor(.white)
                                            .cornerRadius(4)
                                    }
                                    if alreadyHave {
                                        Text("Installed")
                                            .font(.caption2.bold())
                                            .padding(.horizontal, 6)
                                            .padding(.vertical, 2)
                                            .background(Color.green.opacity(0.2))
                                            .foregroundColor(.green)
                                            .cornerRadius(4)
                                    }
                                }
                                Text("\(model.params) · \(model.quant) · \(String(format: "%.1f", model.sizeGB)) GB · min \(model.minRAMGB) GB RAM")
                                    .font(.caption)
                                    .foregroundColor(fits ? .gray : .red.opacity(0.7))
                            }

                            Spacer()

                            if !fits {
                                Image(systemName: "exclamationmark.triangle.fill")
                                    .foregroundColor(.red.opacity(0.5))
                            }
                        }
                        .padding(12)
                        .background(isChosen ? Color.purple.opacity(0.12) : Color.white.opacity(0.04))
                        .cornerRadius(10)
                        .overlay(
                            RoundedRectangle(cornerRadius: 10)
                                .stroke(isChosen ? Color.purple.opacity(0.4) : Color.clear, lineWidth: 1)
                        )
                    }
                    .buttonStyle(.plain)
                    .disabled(!fits)
                }
            }
            .frame(maxWidth: 550)

            HStack(spacing: 16) {
                Button(action: { withAnimation { step = .chooseEngine } }) {
                    Label("Back", systemImage: "arrow.left")
                }
                .buttonStyle(.plain)
                .foregroundColor(.gray)

                Spacer()

                if let model = selectedModel {
                    if hf.isDownloaded(model) {
                        Button(action: {
                            mlx.scanModels()
                            withAnimation { step = .done }
                        }) {
                            Label("Use this model", systemImage: "checkmark.circle.fill")
                                .font(.headline)
                                .padding(.horizontal, 24)
                                .padding(.vertical, 10)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.green)
                    } else {
                        Button(action: {
                            hf.download(model: model)
                            withAnimation { step = .downloading }
                        }) {
                            Label("Download \(String(format: "%.1f", model.sizeGB)) GB", systemImage: "arrow.down.circle.fill")
                                .font(.headline)
                                .padding(.horizontal, 24)
                                .padding(.vertical, 10)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.purple)
                    }
                }
            }
            .frame(maxWidth: 550)
        }
        .padding(40)
    }

    // MARK: - Step 3b: Downloading

    var downloadingStep: some View {
        VStack(spacing: 24) {
            if case .completed = hf.downloadState {
                Image(systemName: "checkmark.circle.fill")
                    .font(.system(size: 56))
                    .foregroundColor(.green)

                Text("Download complete!")
                    .font(.title2.bold())
                    .foregroundColor(.white)

                Text(selectedModel?.name ?? "Model")
                    .font(.title3)
                    .foregroundColor(.purple)

                Button(action: {
                    mlx.scanModels()
                    withAnimation { step = .done }
                }) {
                    Label("Continue", systemImage: "arrow.right.circle.fill")
                        .font(.headline)
                        .padding(.horizontal, 32)
                        .padding(.vertical, 12)
                }
                .buttonStyle(.borderedProminent)
                .tint(.purple)
            } else if case .failed(let msg) = hf.downloadState {
                Image(systemName: "xmark.circle.fill")
                    .font(.system(size: 56))
                    .foregroundColor(.red)

                Text("Download failed")
                    .font(.title2.bold())
                    .foregroundColor(.white)

                Text(msg)
                    .font(.caption)
                    .foregroundColor(.red)

                Button("Retry") {
                    if let model = selectedModel {
                        hf.downloadState = .idle
                        hf.download(model: model)
                    }
                }
                .buttonStyle(.borderedProminent)
                .tint(.purple)

                Button("Back") {
                    hf.downloadState = .idle
                    withAnimation { step = .mlxModelPick }
                }
                .foregroundColor(.gray)
            } else {
                ProgressView()
                    .scaleEffect(1.5)
                    .padding(.bottom, 8)

                Text("Downloading \(selectedModel?.name ?? "model")...")
                    .font(.title3.bold())
                    .foregroundColor(.white)

                Text("\(String(format: "%.1f", selectedModel?.sizeGB ?? 0)) GB from HuggingFace")
                    .font(.callout)
                    .foregroundColor(.gray)

                ScrollView {
                    VStack(alignment: .leading, spacing: 2) {
                        ForEach(Array(hf.downloadLog.suffix(8).enumerated()), id: \.offset) { _, line in
                            Text(line)
                                .font(.caption2.monospaced())
                                .foregroundColor(.gray)
                                .lineLimit(1)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                .frame(maxWidth: 500, maxHeight: 120)
                .padding(8)
                .background(Color.black.opacity(0.3))
                .cornerRadius(8)

                Button("Cancel") {
                    hf.cancelDownload()
                    hf.downloadState = .idle
                    withAnimation { step = .mlxModelPick }
                }
                .font(.caption)
                .foregroundColor(.red.opacity(0.7))
            }
        }
        .padding(40)
    }
}
