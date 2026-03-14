# Simple SF — Quick Ref

## WHAT
Native macOS multi-agent AI app. SwiftUI+Rust FFI. 185 agents. 10 LLM providers. Local-first. Zero server.
51 swift / 34 rust files. ~15K LOC total. All files <500L. SQLite WAL. AGPL-v3.
48 files annotated `// Ref: FT-SSF-XXX`. 40 i18n langs + 6 RTL. TLA+ verified mission engine.

## NEVER
- emoji — SVG Feather/SF Symbols ONLY
- gradient bg . inline styles . hardcoded hex — use SF.Colors/Font/Spacing/Radius tokens
- WebSocket — SSE only . mock/fake data . slop . fallback tests
- `import Foundation` when SwiftUI already imported
- swift build without `cargo build --release` first (Rust .a required)

## Stack
Swift 6 + SwiftUI (macOS 14+) → C FFI (`@_silgen_name`) → Rust staticlib (~30MB .a)
SFEngine: rusqlite+reqwest+tokio+serde+tree-sitter(7 langs)
SimpleSFServer: axum+tower-http (optional REST, port 8099, .gitignore)

## Build / Test
```sh
cd SFEngine && cargo build --release && cd .. && xcrun swift build
cd SFEngine && cargo test       # 7 test files
cd SimpleSFServer && cargo test  # optional server
```

## Tree
```
SimpleSF/                    51 Swift, 7.6K LOC
  App/                       AppState, SimpleSFApp
  Engine/                    SFBridge(432L) +5 ext: Missions, Config, Discussion, Agents, Projects
  Jarvis/                    JarvisView(339) + BubbleView(66) + ToolCallView(284)
  LLM/                       LLMService, Ollama, MLX, HuggingFace, Keychain
  Onboarding/                OnboardingView(311) + StepView + ProgressView + SetupWizard(96) + WizardSteps + FormFields
  Data/                      ChatStore(JSON), ProjectStore, SFCatalog
  Projects/                  ProjectsView(228) + Accordion + EventFeed + Messages + Helpers + Timelines + Constants
  Ideation/                  IdeationView
  Output/                    GitPusher, ZipExporter
  Views/Shared/              MainView ContentView DesignTokens MarkdownView IHMContextHeader SkeletonView LoadingStateView
  Views/Agents/              AgentsView
  Views/Mission/             MissionView(212) + PhaseView(280) + AgentPanel(210)
  i18n/                      LocalizationManager Strings RTLSupport LanguagePickerView
  Resources/SFData/          agents.json(185) patterns.json skills.json workflows.json
  Resources/Locales/         40 JSON locale files (en,fr complete + 38 partial)
SFEngine/                    34 Rust, 7.8K LOC
  src/engine/                10 mod: types discussion workflow mission phase patterns patterns_ext build resilience mod
  src/tools/                 6 mod: code_tools file_tools shell_tools memory_tools schemas mod
  src/indexer/               index_walker index_store + indexer.rs
  src/eval/                  eval_metrics eval_runner + eval.rs
  src/                       llm agents db ffi guard sandbox executor bench catalog ideation protocols lib
SimpleSFServer/              Optional REST (Axum, JWT, CORS restricted, security headers)
formal/                      TLA+ spec: MissionEngine.tla + .cfg (verified: 586 states, 0 errors)
docs/skills/                 5 YAML: UX(636L) A11Y(1244L) Security(531L) Skeleton(1731L) UIComponents(1252L)
docs/wiki/                   13 pages (trace, compliance, security, UX, A11Y, i18n, patterns, LEAN)
traceability.db              E2E SQLite (25 tables, 305 links)
```

## Agents — 185
agents.json. 165 roles. Key teams: brain/worker/code-critic/security-critic/arch-critic/devops/product/tester
Security(10) . SAFe(4) . Feature teams(25) . Platform(10) . RSE(7) . PM(15) . Marketing(6)

## LLM — 10 Providers
Ollama(local) . MLX(local) . OpenAI . Anthropic . Gemini . MiniMax . Kimi . OpenRouter . Qwen . Zhipu
sf_configure_llm() via FFI. RwLock runtime switch. 5 retries exp backoff 2s→60s. Streaming callbacks.

## FFI (15 exports)
sf_init . sf_set_callback . sf_configure_llm . sf_set_yolo
sf_create_project . sf_list_projects . sf_delete_project
sf_start_mission . sf_mission_status . sf_jarvis_discuss . sf_load_discussion_history
sf_start_ideation . sf_list_agents . sf_list_workflows . sf_run_bench . sf_free_string

## Design Tokens (56 — SF enum, DesignTokens.swift)
Colors(39): adaptive(dark:light) NSColor.dynamicProvider
  bg: primary=#0f0a1a/#f5f3f8 secondary=#1a1225/#eae6f0 tertiary=#251d33/#ddd8e6
  brand: purple=#bc8cff/#7c3aed accent=#f78166/#e5603e
  text: primary=#e6edf3/#1a1225 secondary=#9e95b0/#57516a muted=#6e7681/#8b8598
  status: success=#22c55e warning=#f59e0b error=#ef4444 info=#6366f1
  roles: rte(blue) po(green) architect(indigo) lead(amber) dev(cyan) qa(yellow) security(red)
Typo(7): JetBrains Mono 13/11 . System 18b/14sb/13r/11r/10m
Space(5): xs=4 sm=6 md=10 lg=16 xl=24 . Radius(5): sm=4 md=8 lg=12 xl=16 full=999

## UI (22 components)
Atoms(7): AgentAvatarView RoleBadge PatternBadge StatusDot PulseAnimation SkeletonView IHMContextHeader
Molecules(4): MarkdownView SidebarView ContentView LoadingStateView(loading/loaded/empty/error/offline)
Organisms(8): JarvisView ProjectsView MissionView AgentsView IdeationView SetupWizardView OnboardingView MainView
Skeleton: Line Circle Card List Badge + 4 contextual (AgentGrid ProjectList Chat Mission)

## Guard — L0 Adversarial (guard.rs)
25 regex. Score: <5=pass 5-6=soft >=7=reject
SLOP . MOCK . FAKE_BUILD(+7) . HALLUC(claims action w/o tool_calls +5)

## Engine — TLA+ Verified
Patterns: network sequential parallel hierarchical loop aggregator router wave solo
Phases: Once Sprint(PM checkpoint) Gate(loopback→target) FeedbackLoop(QA→tickets→dev)
Resilience: 3 retries exp backoff . LLM health probe . MLX auto-restart
Gate loopback: MAX=3 (TLA+ found infinite loop bug → fixed)
TLA+ proof: 6 safety invariants + 2 liveness properties. 586 states, 0 errors.

## Traceability — E2E UDID
Persona(6) → Feature(25,FT-SSF-NNN) → Story(31,US-SSF-*) → AC(59,AC-SSF-*)
  → IHM(12) → Code(55) → TU(4) → E2E(3) → CRUD(21) → RBAC(20) → Links(305)
traceability.db: 25 tables. Avg coverage: 71%. Test gap: 16%.

## Security (7 critical fixes applied)
FIXED: demo bypass removed . JWT_SECRET env required . CORS restricted(CORS_ORIGINS)
  SQLi parameterized . path traversal(safe_resolve) . security headers . tower-http set-header
SBD 25: 5 PASS 12 WARN 8 FAIL (20%). Priority: rate-limit, CI/CD, model integrity

## Compliance
SOC2: 67% (16/24) . ISO27001: 67% (16/24)

## UX/A11Y/i18n
30 UX laws (lawsofux.com) . 30 WAI-ARIA APG patterns . VoiceOver + keyboard nav
40 langs (en,fr complete) . 6 RTL (ar he fa ur ps ku) . JSON locales . UserDefaults > System > en

## API — 20 Endpoints (SimpleSFServer)
Auth: login/register/me(JWT) . Projects: CRUD+start/stop/pause . Chat: sessions+stream
Ideation: CRUD+stream . LLM: providers+test . Health: /health
JWT_SECRET env required. CORS_ORIGINS env. Security headers enabled.

## DR/GDPR
SQLite: RTO=15min/RPO=0(WAL) . Keychain: RTO=5min/RPO=0 . Chat: RTO=60min/RPO=5min
API keys: Keychain, no retention . Chat: 365d . LLM convos: 90d . Workspace: 365d(Git+ZIP)

## Patterns / Anti-Patterns
DO: local-first . ffi-bridge . wal-sqlite . adversarial-guard . sandbox-exec . streaming-response
  adaptive-theme . design-tokens . keychain-secrets . json-persistence . tree-sitter-index
  skeleton-loading . ihm-context-header . tla+-verification . gate-loopback-cap
DON'T: god-file(>500L) . deep-nesting(>4) . high-coupling . mock-data . slop . fake-build
  hallucination . inline-styles . no-error-handling . spinner-no-context . unbounded-loops

## LEAN/KISS 360° — 55%
PASS(18): all files <500L . trace 71% . compliance 67% . 40 langs . 56 tokens . 185 agents
  7 security fixes . god files split . IHM headers . skeleton loading . TLA+ verified
WARN(20): 21 features no tests . test coverage 16% . 8 SBD fail
FAIL(5): no CI/CD . no rate limiting . no OTEL traces

## Gotchas
- Rust .a BEFORE swift build . Package.swift links -LSFEngine/target/release
- SimpleSFServer in .gitignore — optional component
- macOS 14+(Sonoma) . StrictConcurrency enabled . Swift 6
- JWT_SECRET + CORS_ORIGINS env vars required for server
- Agents: JSON bundle → SQLite at init
- Gate loopback max 3 (TLA+ verified)
