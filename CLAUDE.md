# Simple SF — Quick Ref

## WHAT
Native macOS multi-agent AI app. SwiftUI+Rust FFI. 185 agents. 10 LLM providers. Local-first. Zero server/Docker.
51 swift / 48 rust files. 7.6K Swift LOC + 7.8K Rust LOC. SQLite WAL. AGPL-v3.
All files <500 LOC. 48 files have `// Ref: FT-SSF-XXX` traceability. 40 i18n langs + RTL.

## NEVER
- emoji in UI — SVG Feather icons ONLY
- gradient backgrounds
- inline styles — use SF.Colors/SF.Font/SF.Spacing/SF.Radius tokens
- hardcoded hex colors — use adaptive() helper
- WebSocket — SSE only for streaming
- mock/fake data or tests — live data, real assertions
- `import Foundation` when `import SwiftUI` already imported
- build on CI without `cargo build --release` first (Rust .a required)

## Stack
Swift 6 + SwiftUI (macOS 14+) → C FFI (`@_silgen_name`) → Rust staticlib (~30MB .a)
SFEngine: rusqlite+reqwest+tokio+serde+tree-sitter(7 langs)
SimpleSFServer: axum+tower-http (optional REST API, port 8099)

## Build
```sh
cd SFEngine && cargo build --release && cd ..
xcrun swift build
# Bundle .app
mkdir -p dist/SimpleSF.app/Contents/{MacOS,Resources}
cp .build/arm64-apple-macosx/debug/SimpleSF dist/SimpleSF.app/Contents/MacOS/
cp -R .build/arm64-apple-macosx/debug/SimpleSF_SimpleSF.bundle dist/SimpleSF.app/Contents/Resources/
codesign --force --sign - dist/SimpleSF.app
open dist/SimpleSF.app
```

## Test
```sh
cd SFEngine && cargo test       # 7 test files: integration, eval, chaos, ac_bench, live_e2e, workflow_e2e, pacman
cd SimpleSFServer && cargo test  # server tests (optional)
```

## Tree
```
simple-sf/
  Package.swift              SPM manifest (links Rust .a)
  SimpleSF/                  51 Swift files, 7.6K LOC
    App/                     AppState, SimpleSFApp (entry)
    Engine/                  SFBridge(432L) + 5 extensions: Missions, Config, Discussion, Agents, Projects
    Jarvis/                  JarvisView(339L) + BubbleView(66L) + ToolCallView(284L)
    LLM/                     LLMService, Ollama, MLX, HuggingFace, Keychain
    Onboarding/              OnboardingView(311L) + StepView + ProgressView + SetupWizard(96L) + WizardSteps + FormFields
    Data/                    ChatStore(JSON), ProjectStore, SFCatalog
    Projects/                ProjectsView(228L) + AccordionView + EventFeed + Messages + Helpers + Timelines + Constants
    Ideation/                IdeationView
    Output/                  GitPusher, ZipExporter
    Views/Shared/            MainView, ContentView, DesignTokens, MarkdownView, IHMContextHeader, SkeletonView, LoadingStateView
    Views/Agents/            AgentsView (catalog browser)
    Views/Mission/           MissionView(212L) + PhaseView(280L) + AgentPanel(210L)
    i18n/                    LocalizationManager, Strings, RTLSupport, LanguagePickerView
    Resources/SFData/        agents.json(185) patterns.json skills.json workflows.json
    Resources/Locales/       40 JSON locale files (en,fr complete + 38 partial)
    Resources/Avatars/       22 agent photos (JPG)
  SFEngine/                  48 Rust src files, 7.8K LOC
    src/engine/              10 modules: types, discussion, workflow, mission, phase, patterns, patterns_ext, build, resilience, mod
    src/tools/               6 modules: code_tools, file_tools, shell_tools, memory_tools, schemas, mod
    src/indexer/             3 modules: index_walker, index_store + indexer.rs
    src/eval/                3 modules: eval_metrics, eval_runner + eval.rs
    src/llm.rs               Multi-provider LLM client (10 providers)
    src/agents.rs            Agent CRUD from SQLite
    src/db.rs                SQLite WAL schema
    src/ffi.rs               C FFI exports (15 functions)
    src/guard.rs             L0 adversarial guard (25 patterns)
    src/sandbox.rs           Secure code execution sandbox
    src/executor.rs          Agent execution loop
    src/bench.rs             Performance benchmarks
    src/catalog.rs           Agent catalog loader
    src/ideation.rs          Ideation engine
    src/protocols.rs         Discussion protocols
    src/lib.rs               Crate root
    tests/                   7 test files
  SimpleSFServer/            Optional REST API (Axum, JWT auth, CORS restricted)
    src/main.rs              20 routes, CORS, security headers, health
    src/auth.rs              JWT login/register (no demo bypass)
  docs/
    skills/                  5 deep YAML skills: UX(636L) A11Y(1244L) Security(531L) Skeleton(1731L) UIComponents(1252L)
    wiki/                    13 wiki pages (traceability, compliance, security, UX, A11Y, i18n, patterns, LEAN)
    openapi.json             API spec (20 endpoints)
    security-audit.md        White hat audit report
  traceability.db            E2E traceability SQLite (25 tables, 305 links)
```

## DB — SQLite WAL
SFEngine/src/db.rs — schema: projects, missions, agents(185), discussions, tools, events
traceability.db — audit DB: personas, features, stories, ACs, IHM, code_refs, tests, CRUD, RBAC

## Agents — 185 total
Stored in SimpleSF/Resources/SFData/agents.json. 165 unique roles.
Key: brain(strategic), worker(TDD), code-critic, security-critic, arch-critic, devops, product, tester
Security team(10): pentester-lead, security-researcher, exploit-dev, security-architect, secops, threat-analyst, ciso
SAFe(4): rte, system-architect-art, product-manager-art, chef_de_programme
Feature teams: auth(4), booking(4), payment(3), admin(4), user(4), infra(3), e2e(3), proto(3)
Platform team(10): plat-lead-dev, plat-dev-*, plat-tma-*
RSE(7): rse-manager, rse-eco, rse-nr, rse-ethique-ia, rse-dpo, rse-a11y, rse-juriste, rse-audit-social
PM agents(15): agent-{project} for each SF project
Marketing(6): mkt-cmo, mkt-growth, mkt-brand, mkt-insights, mkt-trend, mkt-bench

## LLM — 10 Providers
Ollama(local) . MLX(local) . OpenAI . Anthropic . Gemini . MiniMax . Kimi . OpenRouter . Alibaba Qwen . Zhipu GLM
Config: sf_configure_llm(provider, api_key, base_url, model) via FFI
Runtime switch: set_model() / set_provider() — RwLock, no restart needed
Retry: 5 retries, exp backoff 2s→60s. Streaming: on_chunk callback. No max_tokens cap.

## FFI Bridge (15 exports)
sf_init . sf_set_callback . sf_configure_llm . sf_set_yolo
sf_create_project . sf_list_projects . sf_delete_project
sf_start_mission . sf_mission_status
sf_jarvis_discuss . sf_load_discussion_history
sf_start_ideation . sf_list_agents . sf_list_workflows
sf_run_bench . sf_free_string

## Design Tokens (56 tokens — SF enum in DesignTokens.swift)
Colors(39): adaptive(dark:light) via NSColor.dynamicProvider
  bg: primary=#0f0a1a/#f5f3f8 secondary=#1a1225/#eae6f0 tertiary=#251d33/#ddd8e6 card=#1e1530/#fff
  brand: purple=#bc8cff/#7c3aed accent=#f78166/#e5603e
  text: primary=#e6edf3/#1a1225 secondary=#9e95b0/#57516a muted=#6e7681/#8b8598
  status: success=#22c55e/#16a34a warning=#f59e0b/#d97706 error=#ef4444/#dc2626 info=#6366f1/#4f46e5
  roles: rte(blue) po(green) architect(indigo) lead(amber) dev(cyan) qa(yellow) devops(blue) security(red) ux(pink)
Typo(7): JetBrains Mono 13/11pt . System 18b/14sb/13r/11r/10m
Space(5): xs=4 sm=6 md=10 lg=16 xl=24
Radius(5): sm=4 md=8 lg=12 xl=16 full=999

## UI Components (22 implemented)
Atoms(7): AgentAvatarView RoleBadge PatternBadge StatusDot PulseAnimation SkeletonView IHMContextHeader
Molecules(4): MarkdownView SidebarView ContentView LoadingStateView(5 states: loading/loaded/empty/error/offline)
Organisms(8): JarvisView ProjectsView MissionView AgentsView IdeationView SetupWizardView OnboardingView MainView
Skeleton: SkeletonLine SkeletonCircle SkeletonCard SkeletonList SkeletonBadge + 4 contextual (AgentGrid, ProjectList, Chat, Mission)
Icons: SF Symbols (system) + Feather SVG. NO emoji.

## Adversarial Guard — L0 Deterministic
guard.rs: 25 regex patterns. Score: <5=pass 5-6=soft >=7=reject
Detects: SLOP(lorem,foo/bar,example.com,TBD,XXX) MOCK(TODO implement,NotImplementedError,fake data)
  FAKE_BUILD(echo SUCCESS,echo Tests passed,exit 0 stub) HALLUC(claims action sans outil)
Hallucination check: claims tool actions but tool_calls[] empty → +5

## Orchestration Patterns
network(multi-agent discussion) . sequential . parallel . hierarchical . loop
Defined in engine.rs — discussion protocol with agent selection, contribution rounds

## Traceability — E2E UDID Chain
```
Persona(6) → Feature(25,FT-SSF-NNN) → Story(31,US-SSF-XXXXXXXX) → AC(59,AC-SSF-XXXXXXXX)
  → IHM(12) → Code(55) → TU(4) → E2E(3) → CRUD(21) → RBAC(20) → Links(305)
```
DB: traceability.db (25 tables). Avg feature coverage: 71%.
Gaps: 21/25 features lack unit tests. Test coverage: 16%.

## Compliance
SOC2: 67% pass (16/24). 8 warn: CC4.1(monitoring), CC7.1(health), CC7.2(IRP), A1.2(DR)
ISO27001: 67% pass (16/24). 4 warn: A.5.24(incident), A.5.34(privacy), A.8.5(MFA), A.8.12(data class)

## Security — SBD 25 Controls (7 critical fixes applied)
FIXED: demo bypass removed . JWT_SECRET env required . CORS restricted(CORS_ORIGINS) . SQL injection parameterized
  path traversal(safe_resolve) . security headers(X-Content-Type/Frame/XSS/Referrer) . tower-http set-header
L1-Input(3): SBD-01=WARN SBD-02=WARN SBD-03=PASS(headers added)
L2-Auth(3): SBD-04=PASS SBD-05=WARN SBD-06=WARN
L3-Data(3): SBD-07=PASS(env secrets) SBD-08=WARN SBD-09=FAIL
L4-Resilience(4): SBD-10=WARN SBD-11=FAIL SBD-12=PASS(safe_resolve) SBD-13=WARN
L5-Supply(12): mostly FAIL — no CI/CD, no model integrity
Score: 20% pass (5/25 pass, 12 warn, 8 fail)

## UX Laws (30 — lawsofux.com)
Perf: Doherty(<400ms) Fitts's(44px targets)
Decision: Hick's(min choices) Choice-Overload Cognitive-Bias Occam's
Memory: Miller(7±2) Cognitive-Load Chunking Mental-Model Working-Memory Serial-Position Zeigarnik
Gestalt: Proximity Similarity Common-Region Pragnanz Uniform-Connectedness
Behavior: Flow Goal-Gradient Peak-End Von-Restorff Active-User Selective-Attention
Strategic: Aesthetic-Usability Jakob's(familiar) Pareto(80/20) Parkinson Tesler Postel

## A11Y — WAI-ARIA APG (30 patterns)
Accordion Alert AlertDialog Breadcrumb Button Carousel Checkbox Combobox Dialog Disclosure
Feed Grid Landmarks Link Listbox Menu MenuBar MenuButton Meter RadioGroup Slider
SliderMultithumb Spinbutton Switch Table Tabs Toolbar Tooltip TreeView Treegrid WindowSplitter
Required: focus-visible . keyboard nav . ARIA roles . semantic views . contrast 4.5:1 . VoiceOver

## i18n — 40 Languages + RTL
Complete(2): en, fr. Partial(38): es de it pt ja ko zh ar ru nl tr pl sv da fi no cs hu ro el he fa ur hi bn th vi id ms tl sw uk bg hr sk am ha yo ig ku ps
RTL(6): ar he fa ur ps ku → layoutDirection(.rightToLeft) + flipsForRightToLeftLayoutDirection
Infra: LocalizationManager(singleton) . Strings.swift(key enum) . RTLSupport.swift . LanguagePickerView
Detection: UserDefaults > System locale > English fallback. JSON locale files in Resources/Locales/

## API — 20 Endpoints (SimpleSFServer)
Auth: POST /api/auth/{login,register} . GET /api/auth/me
Projects: GET/POST /api/projects . GET/PUT/DELETE /api/projects/:id . POST /:id/{start,stop,pause}
Chat: POST /api/chat/sessions . POST /:id/{message,stream} . GET /:id/history
Ideation: GET/POST /api/ideation/sessions . GET /:id . POST /:id/start . GET /:id/stream
LLM: GET /api/providers . POST /api/providers/:name/test
Auth: JWT Bearer. CORS: permissive (WARN).

## GDPR Data Assets
API Keys: restricted, macOS Keychain, no retention
Chat History: internal, JSON+SQLite, 365d
User Credentials: confidential, SQLite server, 365d
LLM Conversations: confidential, 90d
Workspace Files: internal, filesystem, 365d

## DR — RTO/RPO
SQLite DB: critical, RTO=15min/RPO=0 (WAL journal)
API Keys: critical, RTO=5min/RPO=0 (Keychain/iCloud)
Chat History: important, RTO=60min/RPO=5min
Workspace: standard, RTO=240min/RPO=60min (Git+ZIP)

## Patterns (18) / Anti-Patterns (10)
DO: local-first . ffi-bridge . wal-sqlite . adversarial-guard . sandbox-exec . streaming-response
  adaptive-theme . design-tokens . avatar-cache . keychain-secrets . json-persistence . tree-sitter-index
DON'T: god-file(>500L) . deep-nesting(>4) . high-coupling . mock-data . slop-code . fake-build
  hallucination . inline-styles . no-error-handling . spinner-no-context

## LEAN/KISS 360° Score: 55% (up from 12%)
PASS(18): codebase size, all files <500L, trace coverage 71%, 48 files annotated, compliance 67%,
  i18n 40 langs, 56 tokens, 185 agents, 7 security fixes, god files eliminated, IHM headers, skeleton loading
WARN(20): 21 features without tests, test coverage 16%, 8 SBD fail
FAIL(5): no CI/CD pipeline, no rate limiting, no OTEL traces

## Observability (planned)
Traces: mission.execute, phase.execute, agent.invoke, llm.call, tool.call, guard.check, ffi.bridge
Metrics: sf_missions_total, sf_llm_calls_total, sf_llm_latency_ms, sf_guard_rejections
Alerts: MissionStuck(>30min), LLMFailing(>10%), HighRejectRate(>80%), DBCorruption

## Gotchas
- Rust .a must be built BEFORE swift build: `cd SFEngine && cargo build --release`
- Package.swift links via -LSFEngine/target/release
- SimpleSFServer in .gitignore — optional component
- All god files now split — no file >500 LOC
- Agents stored in JSON bundle, loaded into SQLite at init
- macOS 14+ required (Sonoma)
- StrictConcurrency enabled
- JWT_SECRET env var REQUIRED for SimpleSFServer (no fallback)
- CORS_ORIGINS env var controls allowed origins (default: http://localhost:3000)
