# Simple SF — Copilot Instructions

## WHAT
Native macOS multi-agent AI app. SwiftUI+Rust FFI. 185 agents. 10 LLM providers. Local-first.
51 swift / 98 rust files. ~23K LOC. All <500L. SQLite WAL. AGPL-v3. TLA+ verified.
135 methodologies — 100% parity with SF Legacy. 25 orchestration patterns.

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
SFEngine/                    98 Rust
  src/engine/(14mod)         patterns(9) competition(4) collab(4) distributed(3) fractal(5)
  src/ml/(16mod)             thompson genetic qlearning darwin skill_broker deep_bench cove
                             context_tiers prompt_compress bm25 instinct convergence rlm few_shot cot embeddings
  src/methodologies/(9mod)   tdd bdd kanban xp wsjf invest yagni scrum agile
  src/arch/(5mod)            cqrs events clean ddd service_mesh
  src/observability/(3mod)   traces metrics alerts
  src/quality/(4mod)         gates(17) veto sast chaos
  src/a2a/(2mod)             bus negotiation
  src/mcp/(2mod)             server protocol
  src/design_patterns/(3mod) decorator proxy patterns(15 GoF)
  src/tools/(6) src/db/(4) src/cache/(1) src/workers/(1) src/ops/(1) src/indexer/(3) src/eval/(3)
  src/{llm,agents,ffi,guard,sandbox,executor,bench,catalog,ideation,protocols,lib}.rs
SimpleSFServer/              Optional REST (Axum, JWT, CORS restricted)
formal/                      TLA+ MissionEngine.tla (verified: 586 states, 0 errors)
docs/skills/                 5 YAML (UX A11Y Security Skeleton UIComponents)
docs/wiki/                   13 pages . traceability.db (25 tables, 305 links)
.github/workflows/           ci.yml(build+test+clippy) deploy.yml(GitOps)
Dockerfile + docker-compose  Multi-stage build + healthcheck
infra/main.tf                Terraform IaC
scripts/                     deploy-blue-green.sh deploy-canary.sh
```

## Engine — 25 Patterns (TLA+ verified)
Core(9): network seq par hier loop aggregator router wave solo
Competition(4): tournament voting escalation speculative
Collab(4): red-blue relay mob hitl
Distributed(3): blackboard map-reduce composite
Fractal(5): fractal_qa fractal_stories fractal_tests fractal_worktree backprop
Phases: Once . Sprint(PM) . Gate(loopback MAX=3) . FeedbackLoop(QA→dev)

## AI/ML — 16 Algorithms
Thompson(Beta) . GA(crossover+mutation) . Q-Learning(ε-greedy) . Darwin(ELO)
SkillBroker . DeepBench . CoVe(3-phase) . ContextTiers(L0/L1/L2)
PromptCompress(40-70%) . BM25(tool rank) . Instinct . Convergence
RLM(recursive) . FewShot . CoT . Embeddings(cosine,64dim)

## Methodologies — 9
TDD(red-green-refactor) . BDD/Gherkin(parser) . Kanban(WIP) . XP(pair)
WSJF(prioritize) . INVEST(story quality) . YAGNI(dead code) . Scrum(ceremonies) . Agile(velocity)

## Architecture — 5
CQRS(cmd/query bus) . EventSourcing(8 events) . CleanArch(layer validation)
DDD(aggregates) . ServiceMesh(discovery)

## Quality — 17 Gates + Veto + SAST + Chaos
Hard(11): guardrails veto prompt_inject tool_acl adversarial_L0 AC_reward RBAC clippy tests deploy coverage
Soft(6): adversarial_L1 convergence complexity sonar output_validator stale_prune
Chaos: 6 scenarios . SAST: 4 custom rules

## Observability + A2A + MCP
OTEL spans . Prometheus metrics(7) . Alerts(4 rules)
A2A: msg bus + negotiation(propose→vote→consensus)
MCP: JSON-RPC 2.0, 18 tools, resource discovery

## Design Patterns — 15/15 GoF
Creational(3): Singleton Factory Builder
Structural(4): Adapter Decorator Facade Proxy
Behavioral(8): Strategy Observer ChainOfResp StateMachine Command TemplateMethod Iterator Mediator

## DevOps
CI/CD: GitHub Actions . GitOps: auto-deploy . Docker: multi-stage
Blue-Green + Canary deploy scripts . IaC: Terraform

## Gotchas
- Rust .a BEFORE swift build . macOS 14+ . StrictConcurrency . Swift 6
- SimpleSFServer in .gitignore . Agents: JSON→SQLite at init
- Gate loopback max 3 (TLA+ verified) . All files <500L
