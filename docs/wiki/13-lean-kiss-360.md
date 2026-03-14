# LEAN/KISS 360° Quality Report

## Score: 12% (6 pass / 30 warn / 15 fail)

### God Files (>500 LOC) — 14 files

| File | LOC | Action |
|:---|:---:|:---|
| SFEngine/src/engine.rs | 2648 | Split into modules |
| SimpleSF/Projects/ProjectsView.swift | 1761 | Split into modules |
| SFEngine/tests/integration.rs | 1177 | Split into modules |
| SimpleSF/Engine/SFBridge.swift | 1019 | Split into modules |
| SFEngine/tests/ac_bench.rs | 912 | Split into modules |
| SFEngine/src/tools.rs | 900 | Split into modules |
| SFEngine/tests/workflow_e2e.rs | 854 | Split into modules |
| SFEngine/tests/chaos.rs | 743 | Split into modules |
| SimpleSF/Onboarding/OnboardingView.swift | 699 | Split into modules |
| SimpleSF/Views/Mission/MissionView.swift | 680 | Split into modules |
| SimpleSF/Jarvis/JarvisView.swift | 664 | Split into modules |
| SimpleSF/Onboarding/SetupWizardView.swift | 617 | Split into modules |
| SFEngine/src/indexer.rs | 593 | Split into modules |
| SFEngine/src/eval.rs | 584 | Split into modules |

### Test Coverage Gaps — 21/25 features without unit tests

- FT-SSF-001: Jarvis AI Chat
- FT-SSF-002: Multi-Agent Discussions
- FT-SSF-003: Project Management
- FT-SSF-004: Mission Orchestration
- FT-SSF-005: LLM Provider Management
- FT-SSF-006: Ideation Engine
- FT-SSF-007: Onboarding & Setup
- FT-SSF-008: Rich Markdown Rendering
- FT-SSF-009: Chat History & Persistence
- FT-SSF-010: Agent Catalog
- FT-SSF-012: Code Sandbox
- FT-SSF-013: Design System
- FT-SSF-014: Agent Avatars
- FT-SSF-015: i18n Localization
- FT-SSF-016: Git Push Export
- FT-SSF-017: Zip Export
- FT-SSF-018: SQLite Database
- FT-SSF-019: Tool Execution
- FT-SSF-022: Code Indexer
- FT-SSF-023: REST API Server
- FT-SSF-024: Authentication

### Key Metrics

- **Codebase:** 8.3K Swift + 13.6K Rust = 21.7K LOC
- **Traceability:** 71% avg feature coverage (305 links)
- **Test coverage:** 16% features (4/25)
- **Security:** 4% SBD pass (1/25)
- **Compliance:** 67% (SOC2 + ISO27001)
- **Design tokens:** 56 tokens defined
- **i18n:** 12 active / 40 total languages
- **Agents:** 185 cataloged
