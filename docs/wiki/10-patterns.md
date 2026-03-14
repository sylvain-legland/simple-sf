# Patterns & Anti-Patterns

## Patterns (DO)

| ID | Name | Category | Description |
|:---|:---|:---|:---|
| PAT-001 | solo | orchestration | Single agent execution |
| PAT-002 | sequential | orchestration | Agents execute one after another |
| PAT-003 | parallel | orchestration | Agents execute simultaneously |
| PAT-004 | network | orchestration | Agents discuss in a network topology |
| PAT-005 | hierarchical | orchestration | Tree-structured agent hierarchy |
| PAT-006 | loop | orchestration | Iterative refinement loop |
| PAT-007 | local-first | architecture | All processing on device, zero cloud dependency |
| PAT-008 | ffi-bridge | architecture | Swift→C→Rust FFI for performance-critical code |
| PAT-009 | wal-sqlite | architecture | WAL-mode SQLite for crash-safe persistence |
| PAT-010 | adversarial-guard | quality | Deterministic + LLM quality checks |
| PAT-011 | sandbox-exec | security | Isolated code execution environment |
| PAT-012 | streaming-response | ux | Token-by-token LLM response streaming |
| PAT-013 | adaptive-theme | ui | Auto light/dark via NSColor dynamicProvider |
| PAT-014 | design-tokens | ui | Centralized design tokens for consistency |
| PAT-015 | avatar-cache | performance | NSImage cache for agent avatars |
| PAT-016 | keychain-secrets | security | macOS Keychain for API key storage |
| PAT-017 | json-persistence | data | JSON file-based chat history persistence |
| PAT-018 | tree-sitter-index | code | Multi-language code indexing with tree-sitter |

## Anti-Patterns (DON'T)

| ID | Name | Category | Description |
|:---|:---|:---|:---|
| ANTI-001 | god-file | code | Files exceeding 500 LOC |
| ANTI-002 | deep-nesting | code | Nesting deeper than 4 levels |
| ANTI-003 | high-coupling | code | Too many imports/dependencies |
| ANTI-004 | mock-data | quality | Fake/mock data in production |
| ANTI-005 | slop-code | quality | Lorem ipsum, foo/bar, TODO implement |
| ANTI-006 | fake-build | quality | Hardcoded BUILD SUCCESS or Tests passed |
| ANTI-007 | hallucination | quality | Claims actions without tool evidence |
| ANTI-008 | inline-styles | ui | Hardcoded colors/sizes instead of tokens |
| ANTI-009 | no-error-handling | code | Missing error handling in async code |
| ANTI-010 | spinner-no-context | ux | Loading spinner without contextual message |
