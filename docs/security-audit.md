# Security Audit Report — Simple SF

**Date:** 2026-03-14  
**Auditor:** Automated Security Scan (Copilot CLI)  
**Scope:** SFEngine + SimpleSFServer — full codebase review  

---

## 1. CVE Scan — Rust Dependencies

### SFEngine (218 dependencies)

| Crate | Version | CVE | Severity | Description | Fix |
|-------|---------|-----|----------|-------------|-----|
| quinn-proto | 0.11.13 | CVE-2026-31812 / RUSTSEC-2026-0037 | **HIGH (8.7)** | Denial of service in Quinn endpoints — invalid QUIC transport parameters cause panic | Upgrade to ≥0.11.14 |

**Dependency chain:** `quinn-proto 0.11.13 → quinn 0.11.9 → reqwest 0.12.28 → sf-engine 0.1.0`

**Verdict:** ⚠️ WARN — 1 high-severity CVE found. Exploitable via network if QUIC is used by reqwest. Since reqwest uses it as a transport layer for HTTP/3, risk is moderate (HTTP/3 is optional).

### SimpleSFServer (245 dependencies)

No known vulnerabilities found. ✅ PASS

---

## 2. White Hat Security Findings

### 2.1 Authentication — `SimpleSFServer/src/auth.rs`

| # | Finding | Severity | Status |
|---|---------|----------|--------|
| AUTH-01 | **Hardcoded demo bypass password** `"demo2026"` (line 63) allows login to ANY account without knowing the real password | 🔴 CRITICAL | FAIL |
| AUTH-02 | **Hardcoded JWT secret** `"simple-sf-secret-2026"` as fallback (main.rs:44) — any attacker knowing this string can forge valid JWT tokens | 🔴 CRITICAL | FAIL |
| AUTH-03 | JWT token expiry is 30 days (`86400 * 30`) — excessively long for security-sensitive tokens | 🟡 MEDIUM | WARN |
| AUTH-04 | bcrypt with cost 12 — acceptable | ✅ LOW | PASS |
| AUTH-05 | No account lockout after failed login attempts | 🟡 MEDIUM | WARN |
| AUTH-06 | No password complexity validation on registration | 🟡 MEDIUM | WARN |
| AUTH-07 | Demo user seeded with hardcoded bcrypt hash in DB schema (db.rs) | 🟠 HIGH | WARN |

### 2.2 CORS Configuration — `SimpleSFServer/src/main.rs`

| # | Finding | Severity | Status |
|---|---------|----------|--------|
| CORS-01 | **`allow_origin(Any)` + `allow_methods(Any)` + `allow_headers(Any)`** — completely open CORS policy allows any website to make authenticated API requests | 🔴 CRITICAL | FAIL |

### 2.3 Sandbox & Command Execution — `SFEngine/src/sandbox.rs`

| # | Finding | Severity | Status |
|---|---------|----------|--------|
| SAND-01 | Three-tier sandbox design (Docker → macOS sandbox-exec → Direct) — good defense-in-depth | ✅ | PASS |
| SAND-02 | Command allowlist + blocked patterns — solid approach | ✅ | PASS |
| SAND-03 | `curl` and `wget` are in BLOCKED_PATTERNS but `python3 -c "import urllib..."` is in allowlist via `python` prefix — **network exfiltration bypass** via Python | 🟠 HIGH | WARN |
| SAND-04 | Pipeline checking only validates first command's allowlist — `echo ok \| bash -c "malicious"` would fail allowlist but `grep ok \| python3 -c "import os; ..."` could pass | 🟡 MEDIUM | WARN |
| SAND-05 | Docker container runs `--network none` — excellent network isolation | ✅ | PASS |
| SAND-06 | macOS sandbox profile blocks `network-outbound` — good | ✅ | PASS |
| SAND-07 | Memory (512MB), CPU (2), and PID (256) limits in Docker — good | ✅ | PASS |
| SAND-08 | Output truncated at 8KB — prevents memory exhaustion | ✅ | PASS |

### 2.4 Prompt Injection — `SFEngine/src/llm.rs` & `executor.rs`

| # | Finding | Severity | Status |
|---|---------|----------|--------|
| PI-01 | User content is directly concatenated into LLM messages without sanitization — standard for LLM apps but no explicit prompt injection defense | 🟡 MEDIUM | WARN |
| PI-02 | `strip_thinking()` removes `<think>` blocks — mitigates reasoning leakage | ✅ | PASS |
| PI-03 | System prompt is injectable via `executor.rs` line 28 — labeled "protocol injected into system prompt" — appears intentional for mission context | ✅ LOW | PASS |
| PI-04 | No output validation or content filtering on LLM responses before acting on tool calls | 🟡 MEDIUM | WARN |

### 2.5 Command Injection — `SFEngine/src/tools.rs`

| # | Finding | Severity | Status |
|---|---------|----------|--------|
| CMD-01 | `tool_git_commit` uses `format!("git add -A && git commit -m '{}'", msg.replace('\'', "'\\''"))` — shell quoting is correct (single-quote escape) | ✅ | PASS |
| CMD-02 | `tool_build/test/lint` pass user-supplied `command` to `run_shell_allowlisted` — protected by sandbox allowlist | ✅ | PASS |
| CMD-03 | `tool_git_diff` uses `format!("git diff -- '{}' ...")` — path injection possible if path contains single quotes | 🟡 MEDIUM | WARN |
| CMD-04 | All shell execution routed through `sandbox::sandboxed_exec` — centralized control | ✅ | PASS |

### 2.6 Path Traversal — `SFEngine/src/tools.rs`

| # | Finding | Severity | Status |
|---|---------|----------|--------|
| PATH-01 | `code_write`, `code_read`, `code_edit` check for `".."` in path — basic protection present | ✅ | PASS |
| PATH-02 | Check is substring-based (`path.contains("..")`) — does not prevent absolute paths like `/etc/passwd` | 🟠 HIGH | WARN |
| PATH-03 | `list_files` and `deep_search` do NOT check for `".."` or absolute paths — **directory traversal possible** | 🟠 HIGH | WARN |
| PATH-04 | `tool_code_search` `grep_fallback` uses unsanitized pattern in `grep -rn` — regex injection possible (DoS via catastrophic backtracking) | 🟡 MEDIUM | WARN |

### 2.7 SQL Injection — `SFEngine/src/tools.rs`

| # | Finding | Severity | Status |
|---|---------|----------|--------|
| SQL-01 | `tool_memory_search` uses `format!()` to interpolate `project_id` directly into SQL (line 693) — **SQL injection vector** | 🟠 HIGH | WARN |
| SQL-02 | All other queries use `rusqlite::params![]` with parameterized queries — safe | ✅ | PASS |
| SQL-03 | SimpleSFServer `auth.rs` and `db.rs` use parameterized queries throughout | ✅ | PASS |

### 2.8 Hardcoded Secrets

| # | Finding | Severity | Status |
|---|---------|----------|--------|
| SEC-01 | JWT secret fallback `"simple-sf-secret-2026"` in `main.rs:44` | 🔴 CRITICAL | FAIL |
| SEC-02 | Demo bypass password `"demo2026"` in `auth.rs:63` | 🔴 CRITICAL | FAIL |
| SEC-03 | Demo user bcrypt hash hardcoded in DB schema (SimpleSFServer/src/db.rs) | 🟡 MEDIUM | WARN |
| SEC-04 | API keys handled via env vars, not hardcoded — good | ✅ | PASS |
| SEC-05 | LLM API keys stored in memory only (RwLock), not persisted in plaintext | ✅ | PASS |

### 2.9 Guard / Adversarial — `SFEngine/src/guard.rs`

| # | Finding | Severity | Status |
|---|---------|----------|--------|
| GUARD-01 | L0 deterministic checks with scoring system — solid design | ✅ | PASS |
| GUARD-02 | Threshold of 7 for rejection — reasonable | ✅ | PASS |
| GUARD-03 | No L1 LLM-based semantic analysis implemented (only L0 regex) — limited to known patterns | 🟡 MEDIUM | WARN |
| GUARD-04 | Content length check (< 50 chars) has carve-outs for APPROVE/VETO — correct | ✅ | PASS |

---

## 3. Summary by Category

| Category | Status | Critical Issues |
|----------|--------|-----------------|
| **CVE Scan — SFEngine** | ⚠️ WARN | 1 HIGH CVE (quinn-proto DoS) |
| **CVE Scan — SimpleSFServer** | ✅ PASS | None |
| **Authentication** | 🔴 FAIL | Demo bypass password, hardcoded JWT secret |
| **CORS** | 🔴 FAIL | Fully open (`Any` origin/method/header) |
| **Sandbox** | ✅ PASS | Minor bypass via Python stdlib networking |
| **Prompt Injection** | ⚠️ WARN | No input sanitization, no output validation |
| **Command Injection** | ✅ PASS | Protected by sandbox allowlist |
| **Path Traversal** | ⚠️ WARN | Absolute paths not blocked, list_files/deep_search unprotected |
| **SQL Injection** | ⚠️ WARN | 1 format!() interpolation in memory_search |
| **Hardcoded Secrets** | 🔴 FAIL | JWT secret + demo bypass in source code |
| **Guard/Adversarial** | ⚠️ WARN | L0 only, no L1 semantic analysis |

---

## 4. Recommendations

### 🔴 Critical — Fix Immediately

1. **Remove demo bypass password** (`auth.rs:63`): Delete `|| req.password == "demo2026"`. This allows authentication bypass for any account.

2. **Remove hardcoded JWT secret** (`main.rs:44`): The fallback `"simple-sf-secret-2026"` must be removed. Require `JWT_SECRET` env var or generate a random secret at startup.

3. **Restrict CORS** (`main.rs`): Replace `allow_origin(Any)` with an explicit allowlist of trusted origins. At minimum use the server's own origin.

### 🟠 High — Fix Soon

4. **Fix SQL injection** in `tools.rs` `tool_memory_search`: Replace `format!()` SQL with parameterized query for `project_id`.

5. **Block absolute paths** in file tools: Add check `if path.starts_with("/")` alongside the `".."` check. Apply to `list_files` and `deep_search` too.

6. **Block Python network access** in sandbox: Add `"import urllib"`, `"import requests"`, `"import http"`, `"import socket"` to BLOCKED_PATTERNS.

7. **Upgrade quinn-proto** to ≥0.11.14 in SFEngine: Run `cargo update -p quinn-proto`.

### 🟡 Medium — Improve

8. **Reduce JWT expiry** from 30 days to 24 hours with refresh token support.

9. **Add account lockout** after 5 failed login attempts (rate-limit per email).

10. **Add password complexity requirements** on registration (min 8 chars, mixed case, etc.).

11. **Add prompt injection defense**: Sanitize user content before LLM submission (strip system-prompt-like instructions).

12. **Add LLM output validation**: Check tool call arguments for suspicious patterns before execution.

13. **Implement L1 guard**: Add LLM-based semantic analysis for adversarial detection.

14. **Sanitize grep patterns** in `grep_fallback`: Escape regex special characters or use fixed-string mode (`-F`).

---

## 5. Audit Metadata

- **Cargo audit DB:** 949 advisories (2026-03-14)
- **SFEngine dependencies scanned:** 218 crates
- **SimpleSFServer dependencies scanned:** 245 crates
- **Source files reviewed:** sandbox.rs, auth.rs, llm.rs (×2), tools.rs, guard.rs, main.rs, db.rs (×2), executor.rs, projects.rs
- **Tools used:** `cargo audit`, manual code review, regex pattern scanning
