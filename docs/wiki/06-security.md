# Security — SecureByDesign 25 Controls

## L1-Input

| Control | Name | Status | Evidence |
|:---|:---|:---:|:---|
| SBD-01 | Input Validation & Sanitization | WARN | Path traversal: ".." checked but absolute paths not blocked. list_files/deep_search lack path validation. grep_fallback allows regex injection. |
| SBD-02 | Prompt Injection Defense | WARN | User content passed directly to LLM without sanitization. No prompt injection defense. No output validation on tool calls. |
| SBD-03 | Output Encoding & Content Security | FAIL | CORS: allow_origin(Any) + allow_methods(Any) + allow_headers(Any). No CSP headers. Any website can make authenticated requests. |

## L2-Auth

| Control | Name | Status | Evidence |
|:---|:---|:---:|:---|
| SBD-04 | Authentication Integrity | FAIL | CRITICAL: Hardcoded demo bypass password "demo2026" (auth.rs:63) allows login to ANY account. Hardcoded JWT secret fallback "simple-sf-secret-2026" (main.rs:44). No account lockout. No password complexity rules. bcrypt cost=12 is OK. |
| SBD-05 | Authorization & Access Control | WARN | JWT contains role but no middleware enforces role-based access on API routes. All authenticated users have equal access. |
| SBD-06 | Least Privilege | PASS | Sandbox uses command allowlist + blocked patterns. Docker --network none. macOS sandbox-exec with deny-default profile. Minor: Python stdlib networking bypass possible. |

## L3-Data

| Control | Name | Status | Evidence |
|:---|:---|:---:|:---|
| SBD-07 | Secrets Management | FAIL | CRITICAL: JWT secret hardcoded as fallback in source code. Demo user bcrypt hash in schema. API keys properly via env vars. |
| SBD-08 | Cryptographic Standards | PASS | JWT uses HS256 (acceptable for symmetric). bcrypt cost=12 for passwords. reqwest uses rustls-tls. |
| SBD-09 | Sensitive Data Minimization | FAIL | Full chat history stored verbatim (chat.rs:70-71). No data classification, retention policy, or PII detection. Project metadata, agent discussions, and system prompts retained indefinitely. No anonymization. |

## L4-Resilience

| Control | Name | Status | Evidence |
|:---|:---|:---:|:---|
| SBD-10 | Security Logging & Audit Trail | WARN | Tracing subscriber initialized (main.rs:23). Server startup logged (main.rs:78). No login/logout audit logs. No action tracking for CRUD. Auth failures return generic message but no audit trail. Ideation failures logged (ideation.rs:45). |
| SBD-11 | Rate Limiting & Abuse Prevention | FAIL | No rate limiting on any endpoint. Login endpoint vulnerable to brute force. LLM endpoints have no abuse prevention. |
| SBD-12 | SSRF Prevention | FAIL | No URL allowlist validation. No internal IP range blocking. External URLs accepted without validation. Exploitable if user can configure custom provider URLs. |
| SBD-13 | Error Handling & Information Disclosure | WARN | Database errors returned as strings (projects.rs:73). LLM errors include full response text (llm.rs:161). Generic "Invalid credentials" for auth (auth.rs:66,76). No stack traces in API responses. |

## L5-Supply

| Control | Name | Status | Evidence |
|:---|:---|:---:|:---|
| SBD-14 | Dependency & Supply Chain Security | WARN | SFEngine: 1 HIGH CVE (RUSTSEC-2026-0037, quinn-proto 0.11.13, DoS). SimpleSFServer: clean. Fix: cargo update -p quinn-proto. |
| SBD-15 | CI/CD Pipeline Integrity | FAIL | No CI/CD pipeline implemented. No .github/workflows or GitHub Actions. Manual build process documented in README. No signed commits or releases. |
| SBD-16 | LLM Supply Chain & Model Integrity | FAIL | No hash verification for downloaded models (Ollama/MLX). LLM responses not validated for tampering. Agent definitions loaded from DB without checksums (catalog.rs:88-116). |
| SBD-17 | System Prompt Protection | WARN | System prompts are not protected from extraction. No defense against prompt leaking via instruction injection. |
| SBD-18 | RAG & Embedding Security | WARN | Code search tool present (tools.rs:101-115). Project memory storage/retrieval (tools.rs:767). No access control on memory_search/memory_store by role or ownership. All authenticated users can query any project memory. |
| SBD-19 | LLM Output Validation | WARN | L0 guard checks for slop/hallucination patterns. No L1 semantic analysis. Tool call arguments not validated before execution. |
| SBD-20 | Network Architecture & CORS | FAIL | CORS completely open (Any/Any/Any). Server binds to 127.0.0.1 only (good). No TLS termination configured. |
| SBD-21 | Secure Design Principles (Fail Secure) | WARN | Default auth fallback returns 401 (auth.rs:124). Default database to local machine (main.rs:30-35). YOLO_MODE defaults to false (engine.rs:18). Sandbox defaults Docker->macOS->Direct (sandbox.rs:46-64). Hardcoded demo password allows bypass. |
| SBD-22 | Governance & Security Posture | FAIL | AGPL license present. No SECURITY.md. No vulnerability disclosure policy. No security review checklist or governance documentation. |
| SBD-23 | Asset Inventory & Configuration Management | WARN | Cargo.lock files track all dependencies. Agent catalog in JSON (agents.json). No component SBOM. No external service inventory. 10 LLM providers supported but not formally documented. |
| SBD-24 | Incident Response Readiness | FAIL | No structured error reporting or alerting. No security event classification. No incident response runbooks. Engine recognizes incident role only for workflow routing (engine.rs:382-387). |
| SBD-25 | Privacy & Compliance by Design | FAIL | Full chat history retained indefinitely. No user data export or deletion endpoints. No consent tracking or privacy policy links. Demo user hardcoded (db.rs:91-93). No GDPR right to be forgotten implementation. |


**Summary:** 2 pass / 11 warn / 12 fail out of 25 controls
