# Simple SF — Quick Ref

## WHAT
Native macOS multi-agent AI app. SwiftUI+Rust FFI. 185 agents. 10 LLM providers. Local-first. Zero server/Docker.
25 swift / 32 rust files. 8.3K Swift LOC + 13.6K Rust LOC. SQLite WAL. AGPL-v3.

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
  SimpleSF/                  25 Swift files, 8.3K LOC
    App/                     AppState, SimpleSFApp (entry)
    Engine/                  SFBridge.swift (1019L — Swift↔Rust FFI)
    Jarvis/                  JarvisView.swift (chat UI)
    LLM/                     LLMService, Ollama, MLX, HuggingFace, Keychain
    Onboarding/              OnboardingView, SetupWizardView
    Data/                    ChatStore(JSON), ProjectStore, SFCatalog
    Projects/                ProjectsView (CRUD)
    Ideation/                IdeationView
    Output/                  GitPusher, ZipExporter
    Views/Shared/            MainView, ContentView, DesignTokens, MarkdownView
    Views/Agents/            AgentsView (catalog browser)
    Views/Mission/           MissionView (orchestration)
    Resources/SFData/        agents.json(185) patterns.json skills.json workflows.json
    Resources/Avatars/       22 agent photos (JPG)
    i18n/                    Localizable.xcstrings (12 langs)
  SFEngine/                  17 Rust src files, 13.6K LOC
    src/engine.rs            Discussion orchestrator (2648L — GOD_FILE)
    src/llm.rs               Multi-provider LLM client (10 providers)
    src/agents.rs            Agent CRUD from SQLite
    src/db.rs                SQLite WAL schema (projects, missions, agents, discussions, tools)
    src/ffi.rs               C FFI exports (15 functions)
    src/guard.rs             L0 adversarial guard (25 pattern checks)
    src/sandbox.rs           Secure code execution sandbox
    src/tools.rs             Agent tool implementations
    src/executor.rs          Agent execution loop
    src/indexer.rs           Tree-sitter code indexer (7 langs)
    src/eval.rs              Output evaluation engine
    src/bench.rs             Performance benchmarks
    src/catalog.rs           Agent catalog loader
    src/ideation.rs          Ideation engine
    src/protocols.rs         Discussion protocols
    src/lib.rs               Crate root
    tests/                   7 test files
  SimpleSFServer/            Optional REST API (Axum, JWT auth)
    src/main.rs              20 routes, CORS, health
    src/auth.rs              JWT login/register
    src/chat.rs              Chat sessions + SSE streaming
    src/projects.rs          Project CRUD
    src/ideation.rs          Ideation sessions
  docs/
    skills/                  Deep YAML skills (UX, A11Y, Security, UI)
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

## UI Components (16 implemented)
Atoms(5): AgentAvatarView RoleBadge PatternBadge StatusDot PulseAnimation
Molecules(3): MarkdownView SidebarView ContentView
Organisms(8): JarvisView ProjectsView MissionView AgentsView IdeationView SetupWizardView OnboardingView MainView
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

## Security — SBD 25 Controls
L1-Input(3): SBD-01 validation=WARN . SBD-02 prompt-inject=WARN . SBD-03 CSP=FAIL
L2-Auth(3): SBD-04 auth=PASS . SBD-05 authz=WARN . SBD-06 least-priv=WARN
L3-Data(3): SBD-07 secrets=WARN . SBD-08 crypto=WARN . SBD-09 minimize=FAIL
L4-Resilience(4): SBD-10 logging=WARN . SBD-11 rate-limit=FAIL . SBD-12 SSRF=FAIL . SBD-13 errors=WARN
L5-Supply(12): mostly FAIL — no CI/CD, no model integrity, no CORS policy
Score: 4% pass (1/25 pass, 14 warn, 10 fail)
Priority: 1.rate-limit 2.CORS 3.security-headers 4.CI/CD 5.secrets-mgmt

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
Active(12): en fr es de it pt ja ko zh ar ru nl
Planned(28): tr pl sv da fi no cs hu ro el he fa ur hi bn th vi id ms tl sw uk bg hr sk am ha yo
RTL(4): ar he fa ur → layoutDirection(.rightToLeft) + flipsForRightToLeftLayoutDirection

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

## LEAN/KISS 360° Score: 12%
PASS(6): codebase size, trace coverage 71%, compliance 67%, i18n 12 langs, 56 tokens, 185 agents
WARN(30): 8 large files(300-500L), 21 features without tests, test coverage 16%
FAIL(15): 14 god files(>500L), security 4% pass

## Observability (planned)
Traces: mission.execute, phase.execute, agent.invoke, llm.call, tool.call, guard.check, ffi.bridge
Metrics: sf_missions_total, sf_llm_calls_total, sf_llm_latency_ms, sf_guard_rejections
Alerts: MissionStuck(>30min), LLMFailing(>10%), HighRejectRate(>80%), DBCorruption

## Gotchas
- Rust .a must be built BEFORE swift build: `cd SFEngine && cargo build --release`
- Package.swift links via -LSFEngine/target/release
- SimpleSFServer in .gitignore — optional component
- engine.rs is 2648 LOC — needs splitting
- SFBridge.swift is 1019 LOC — large FFI layer
- ProjectsView.swift 1761 LOC — needs decomposition
- Agents stored in JSON bundle, loaded into SQLite at init
- macOS 14+ required (Sonoma)
- StrictConcurrency enabled
