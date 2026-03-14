# Simple SF — Copilot Instructions

## WHAT
Native macOS multi-agent AI app. SwiftUI+Rust FFI. 185 agents. 10 LLM providers. Local-first.
25 swift / 32 rust files. 8.3K+13.6K LOC. SQLite WAL. AGPL-v3.

## NEVER
- emoji in UI — SVG Feather/SF Symbols ONLY
- gradient bg . inline styles . hardcoded hex — use SF.Colors/Font/Spacing/Radius tokens
- WebSocket — SSE only . mock/fake data . slop code
- build Swift without Rust .a first: `cd SFEngine && cargo build --release`

## Stack
Swift 6+SwiftUI(macOS14) → C FFI(@_silgen_name) → Rust staticlib
SFEngine: rusqlite+reqwest+tokio+serde+tree-sitter(7 langs)
SimpleSFServer: axum+tower-http (optional, port 8099)

## Build / Test
```sh
cd SFEngine && cargo build --release && cd ..
xcrun swift build
cd SFEngine && cargo test  # 7 test files
```

## Tree
```
SimpleSF/                    25 Swift files
  App/ Engine/ Jarvis/ LLM/ Onboarding/ Data/ Projects/ Ideation/ Output/
  Views/{Shared,Agents,Mission}/ Resources/{SFData,Avatars}/ i18n/
SFEngine/                    17 Rust src + 7 test files
  src/{engine,llm,agents,db,ffi,guard,sandbox,tools,executor,indexer,eval,bench,catalog,ideation,protocols,lib}.rs
SimpleSFServer/              Optional REST API (Axum)
docs/skills/                 Deep YAML skills (UX, A11Y, Security, UI)
traceability.db              E2E audit DB (25 tables, 305 links)
```

## Design Tokens (SF enum — DesignTokens.swift)
Colors(39): adaptive(dark:light) NSColor.dynamicProvider
  bg: primary=#0f0a1a/#f5f3f8 . brand: purple=#bc8cff/#7c3aed accent=#f78166/#e5603e
  text: primary=#e6edf3/#1a1225 . status: success/warning/error/info
  roles: rte(blue) po(green) architect(indigo) lead(amber) dev(cyan) qa(yellow) security(red)
Typo(7): JetBrains Mono 13/11 . System 18b/14sb/13r/11r/10m
Space(5): xs=4 sm=6 md=10 lg=16 xl=24 . Radius(5): sm=4 md=8 lg=12 xl=16 full=999

## LLM — 10 Providers
Ollama . MLX . OpenAI . Anthropic . Gemini . MiniMax . Kimi . OpenRouter . Qwen . Zhipu
Runtime switch via RwLock. 5 retries exp backoff 2s→60s. Streaming callbacks.

## FFI (15 exports)
sf_init . sf_configure_llm . sf_create_project . sf_list_projects . sf_delete_project
sf_start_mission . sf_mission_status . sf_jarvis_discuss . sf_load_discussion_history
sf_start_ideation . sf_list_agents . sf_list_workflows . sf_run_bench . sf_free_string

## Guard — L0 Adversarial (guard.rs)
25 regex patterns. Score: <5=pass 5-6=soft >=7=reject
SLOP . MOCK . FAKE_BUILD(+7) . HALLUC(claims action w/o tool_calls)

## Traceability (traceability.db)
Persona(6) → Feature(25) → Story(31) → AC(59) → IHM(12) → Code(55) → TU(4) → E2E(3) → CRUD(21) → RBAC(20)
305 links. Avg coverage: 71%. Test coverage: 16% (gap).

## Compliance
SOC2: 67% (16/24 pass, 8 warn) . ISO27001: 67% (16/24 pass, 8 warn)
Security SBD: 4% (1/25 pass, 14 warn, 10 fail) — priority: rate-limit, CORS, headers, CI/CD

## UX (30 laws) . A11Y (30 WAI-ARIA patterns) . i18n (40 langs, 4 RTL)
Skills in docs/skills/*.yaml

## API — 20 Endpoints
Auth: login/register/me . Projects: CRUD+start/stop/pause . Chat: sessions+stream . Ideation: CRUD+stream
LLM: providers+test . JWT Bearer. Health: /health

## Gotchas
- Rust .a BEFORE swift build . engine.rs=2648L(god file) . SFBridge=1019L . ProjectsView=1761L
- Agents in JSON bundle → SQLite at init . macOS 14+ . StrictConcurrency enabled
- SimpleSFServer in .gitignore
