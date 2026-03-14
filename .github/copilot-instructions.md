# Simple SF — Copilot Instructions

## WHAT
Native macOS multi-agent AI app. SwiftUI+Rust FFI. 185 agents. 10 LLM providers. Local-first.
51 swift / 34 rust files. ~15K LOC. All <500L. SQLite WAL. AGPL-v3. TLA+ verified engine.

## NEVER
- emoji — SVG Feather/SF Symbols ONLY
- gradient bg . inline styles . hardcoded hex — use SF.Colors/Font/Spacing/Radius tokens
- WebSocket — SSE only . mock/fake data . slop . fallback tests
- swift build without `cargo build --release` first

## Stack
Swift 6+SwiftUI(macOS14) → C FFI(@_silgen_name) → Rust staticlib
SFEngine: rusqlite+reqwest+tokio+serde+tree-sitter(7 langs)
SimpleSFServer: axum+tower-http (optional, port 8099, .gitignore)

## Build / Test
```sh
cd SFEngine && cargo build --release && cd .. && xcrun swift build
cd SFEngine && cargo test  # 7 test files
```

## Tree
```
SimpleSF/                    51 Swift
  App/ Engine/(SFBridge+5ext) Jarvis/(3) LLM/(5) Onboarding/(6) Data/(3)
  Projects/(8) Ideation/ Output/(2) Views/{Shared(7),Agents,Mission(3)} i18n/(4)
  Resources/{SFData(4json),Locales(40json),Avatars(22jpg)}
SFEngine/                    34 Rust
  src/engine/(10mod) src/tools/(6mod) src/indexer/(3) src/eval/(3)
  src/{llm,agents,db,ffi,guard,sandbox,executor,bench,catalog,ideation,protocols,lib}.rs
SimpleSFServer/              Optional REST (Axum, JWT, CORS restricted)
formal/                      TLA+ MissionEngine.tla (verified: 586 states, 0 errors)
docs/skills/                 5 YAML (UX A11Y Security Skeleton UIComponents)
docs/wiki/                   13 pages . traceability.db (25 tables, 305 links)
```

## Tokens (SF enum — DesignTokens.swift)
Colors(39): adaptive(dark:light) . bg: #0f0a1a/#f5f3f8 . brand: purple=#bc8cff . text: #e6edf3/#1a1225
  status: success/warning/error/info . roles: rte/po/architect/lead/dev/qa/security
Typo(7): JetBrains Mono 13/11 . System 18b/14sb/13r/11r/10m
Space(5): xs=4 sm=6 md=10 lg=16 xl=24 . Radius(5): sm=4 md=8 lg=12 xl=16 full=999

## LLM — 10
Ollama . MLX . OpenAI . Anthropic . Gemini . MiniMax . Kimi . OpenRouter . Qwen . Zhipu
RwLock runtime switch. 5 retries exp backoff 2s→60s. Streaming callbacks.

## FFI (15)
sf_init . sf_configure_llm . sf_set_yolo . sf_create/list/delete_project
sf_start_mission . sf_mission_status . sf_jarvis_discuss . sf_load_discussion_history
sf_start_ideation . sf_list_agents . sf_list_workflows . sf_run_bench . sf_free_string

## Guard — L0 (guard.rs)
25 regex. <5=pass 5-6=soft >=7=reject. SLOP MOCK FAKE_BUILD(+7) HALLUC(+5)

## Engine — TLA+ Verified
Patterns: network seq par hier loop aggregator router wave solo
Phases: Once . Sprint(PM checkpoint) . Gate(loopback MAX=3) . FeedbackLoop(QA→tickets→dev)
Resilience: 3 retries . LLM probe . MLX auto-restart
Proof: 6 invariants + 2 liveness. 586 states 0 errors.

## Traceability (traceability.db)
Persona(6)→Feature(25)→Story(31)→AC(59)→IHM(12)→Code(55)→TU(4)→E2E(3)→CRUD(21)→RBAC(20)
305 links. 71% coverage. 48 files annotated `// Ref: FT-SSF-XXX`. Test gap: 16%.

## Security (7 fixes applied)
demo bypass removed . JWT_SECRET env . CORS restricted . SQLi parameterized
path traversal(safe_resolve) . security headers . SBD: 5/25 PASS 12 WARN 8 FAIL

## Compliance
SOC2: 67%(16/24) . ISO27001: 67%(16/24) . LEAN/KISS: 55%

## UX(30 laws) . A11Y(30 ARIA) . i18n(40 langs, 6 RTL)
Skills: docs/skills/{ux-laws,a11y-wai-aria,secure-by-design,ui-skeleton-annotation,ui-components}-deep.yaml

## API — 20 endpoints (SimpleSFServer)
Auth(JWT) . Projects(CRUD+control) . Chat(sessions+stream) . Ideation(CRUD+stream) . LLM(providers)
JWT_SECRET + CORS_ORIGINS env required. Security headers enabled.

## Gotchas
- Rust .a BEFORE swift build . macOS 14+ . StrictConcurrency . Swift 6
- SimpleSFServer in .gitignore . Agents: JSON→SQLite at init
- Gate loopback max 3 (TLA+ verified) . All files <500L
