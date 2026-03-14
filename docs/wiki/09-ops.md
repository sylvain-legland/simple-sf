# Operations — Observability, API, GDPR, DR

## Observability

### Traces (7)

| ID | Name | Description | Target |
|:---|:---|:---|:---|
| OBS-TR-001 | mission.execute | End-to-end mission execution span | SFEngine |
| OBS-TR-002 | phase.execute | Individual phase execution span | SFEngine |
| OBS-TR-003 | agent.invoke | Agent invocation span | SFEngine |
| OBS-TR-004 | llm.call | LLM API call span with model/provider | SFEngine |
| OBS-TR-005 | tool.call | Tool execution span | SFEngine |
| OBS-TR-006 | guard.check | Adversarial guard check span | SFEngine |
| OBS-TR-007 | ffi.bridge | Swift↔Rust FFI call span | SFBridge |

### Metrics (7)

| ID | Name | Description | Target |
|:---|:---|:---|:---|
| OBS-MT-001 | sf_missions_total | Total missions started (counter) | SFEngine |
| OBS-MT-002 | sf_llm_calls_total | Total LLM API calls (counter by provider) | SFEngine |
| OBS-MT-003 | sf_llm_latency_ms | LLM call latency histogram | SFEngine |
| OBS-MT-004 | sf_guard_rejections | Guard rejection counter by type | SFEngine |
| OBS-MT-005 | sf_agent_executions | Agent execution counter by role | SFEngine |
| OBS-MT-006 | sf_tool_calls | Tool call counter by tool name | SFEngine |
| OBS-MT-007 | sf_db_queries | Database query counter | SFEngine |

### Alerts (4)

| ID | Name | Description | Target |
|:---|:---|:---|:---|
| OBS-AL-001 | MissionStuck | Mission running > 30min without progress | SFEngine |
| OBS-AL-002 | LLMFailing | LLM error rate > 10% in 5min window | SFEngine |
| OBS-AL-003 | HighRejectRate | Guard rejection rate > 80% | SFEngine |
| OBS-AL-004 | DBCorruption | SQLite integrity check failed | SFEngine |

## GDPR Data Assets

| ID | Name | Classification | Retention | Basis | Location |
|:---|:---|:---|:---|:---|:---|
| DA-001 | API Keys | restricted | 0d | consent | macOS Keychain |
| DA-002 | Chat History | internal | 365d | legitimate_interest | SQLite + JSON |
| DA-003 | Project Data | internal | 365d | legitimate_interest | SQLite |
| DA-004 | Mission Results | internal | 180d | legitimate_interest | SQLite |
| DA-005 | Agent Catalog | public | 0d | legitimate_interest | JSON bundle |
| DA-006 | User Credentials | confidential | 365d | consent | SQLite (server) |
| DA-007 | LLM Conversations | confidential | 90d | consent | SQLite + JSON |
| DA-008 | Workspace Files | internal | 365d | legitimate_interest | Filesystem |
| DA-009 | Ideation Sessions | internal | 180d | legitimate_interest | SQLite |
| DA-010 | Benchmark Results | internal | 90d | legitimate_interest | SQLite |

## Disaster Recovery

| ID | Component | Tier | RTO | RPO | Failover | Backup |
|:---|:---|:---|:---|:---|:---|:---|
| DR-001 | SQLite Database | critical | 15min | 0min | WAL mode + local backup | SQLite WAL journal |
| DR-002 | Chat History JSON | important | 60min | 5min | JSON file copy | Filesystem backup |
| DR-003 | API Keys (Keychain) | critical | 5min | 0min | macOS Keychain restore | Keychain backup/iCloud |
| DR-004 | LLM Provider Config | important | 30min | 5min | Re-configure from settings | UserDefaults |
| DR-005 | Workspace Files | standard | 240min | 60min | Git restore or ZIP backup | Git + ZIP export |
| DR-006 | Agent Catalog | standard | 5min | 0min | Bundled with app | App bundle immutable |
| DR-007 | Server (SimpleSFServer) | important | 60min | 5min | Restart process | Systemd/launchd |
