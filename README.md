# simple-sf

**Simple Software Factory** — macOS native app with the full SF platform embedded.

## What

A clean macOS SwiftUI app that bundles the entire Software Factory Python backend inside the `.app` — no server setup, no Docker, no external dependencies. Just open the app.

### Features
- **Jarvis** — conversational AI assistant (streaming)
- **Ideation** — AI teams brainstorming in parallel
- **Projects** — list with progress bar, start / pause / stop
- **Full SF** — all 133+ agents, 12 patterns, SAFe, A2A, RLM available in Advanced mode
- **8 LLM providers** — OpenRouter, OpenAI, Anthropic, Gemini, Kimi, MiniMax, Qwen, GLM
- **40 languages** — full i18n
- **Output** — ZIP export or Git push to GitHub / GitLab

### Simple ↔ Advanced mode
Toggle in the toolbar. Simple = 3 views. Advanced = full SF platform.

## Requirements

- macOS 14+
- Xcode 15+

## Setup

```bash
# 1. Bundle Python + SF backend (one-time, ~5 min)
./Scripts/embed_python.sh

# 2. Open in Xcode
open SimpleSF.xcodeproj
```

The `embed_python.sh` script downloads a standalone Python 3.12, installs all dependencies, and copies the SF platform code into `SimpleSF/Resources/`. This folder is gitignored — the bundle is built locally.

## Architecture

```
SimpleSF.app/
├── MacOS/SimpleSF              ← Swift binary
├── Frameworks/Python.framework ← Python 3.12 runtime
└── Resources/
    ├── platform/               ← SF Python codebase
    ├── site-packages/          ← pip dependencies
    └── *.lproj/                ← 40 language bundles
```

SwiftUI → URLSession → http://127.0.0.1:{port} → FastAPI (embedded)

## LLM Keys

API keys are stored in the macOS Keychain (never in files). Configure them in the Onboarding screen on first launch.
