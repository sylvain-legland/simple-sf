# Compliance — SOC2 & ISO27001

## SOC2 — 8/12 pass, 4 warn

| Control | Name | Status | Evidence |
|:---|:---|:---:|:---|
| A1.2 | Disaster Recovery | WARN | SQLite WAL, no formal DR |
| CC1.1 | COSO Principle 1 — Integrity & Ethics | PASS | Code review, adversarial guard, anti-slop checks |
| CC1.2 | COSO Principle 2 — Board Oversight | PASS | CTO agent in 185-agent hierarchy |
| CC2.1 | Communication & Information | PASS | Agent discussions, structured output, audit log |
| CC3.1 | Risk Assessment | PASS | L0 adversarial guard, 25+ pattern checks |
| CC4.1 | Monitoring Activities | WARN | Benchmarks exist but no continuous monitoring |
| CC5.1 | Control Activities | PASS | Sandbox, tool ACL, guard checks |
| CC6.1 | Logical Access | PASS | JWT auth with login/register |
| CC6.6 | System Boundaries | PASS | Sandbox isolation, FFI boundary |
| CC7.1 | System Monitoring | WARN | Health endpoint exists, no alerting |
| CC7.2 | Incident Response | WARN | Guard rejection exists, no formal IRP |
| CC8.1 | Change Management | PASS | Git integration, version tracking |

## ISO27001 — 8/12 pass, 4 warn

| Control | Name | Status | Evidence |
|:---|:---|:---:|:---|
| A.5.1 | Information Security Policies | PASS | Guard module, sandbox, auth |
| A.5.15 | Access Control | PASS | JWT + RBAC rules defined |
| A.5.17 | Authentication Information | PASS | JWT tokens, password hashing |
| A.5.24 | Incident Management | WARN | Guard detects issues, no formal IRP |
| A.5.34 | Privacy Protection | WARN | Local-first reduces exposure, no GDPR policy |
| A.5.8 | Information Security in Project Mgmt | PASS | Adversarial guard per phase |
| A.8.1 | Asset Management | PASS | 185 agents cataloged, DB schema defined |
| A.8.12 | Data Classification | WARN | No formal classification |
| A.8.25 | Secure Development | PASS | L0 guard, sandbox, code review agents |
| A.8.28 | Secure Coding | PASS | Tree-sitter indexing, guard patterns |
| A.8.5 | Secure Authentication | WARN | JWT only, no MFA |
| A.8.9 | Configuration Management | PASS | LLM config, project config in DB |

