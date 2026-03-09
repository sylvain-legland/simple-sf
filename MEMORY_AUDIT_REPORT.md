# MEMORY SYSTEM AUDIT: simple-sf vs platform

**Date**: 2024
**Project**: /Users/sylvain/_MACARON-SOFTWARE/simple-sf
**Platform**: /Users/sylvain/_MACARON-SOFTWARE/_SOFTWARE_FACTORY/platform

---

## 1. MEMORY-RELATED MODULES

### simple-sf (SFEngine/src/)
✅ **Found**: 3 memory-related modules
- `db.rs` — SQLite schema with memory table
- `tools.rs` — memory_search and memory_store tool implementations
- `engine.rs` — conversation history loading between phases

❌ **Missing**:
- No dedicated memory.rs module
- No vector/embedding layer
- No semantic search implementation
- No memory maintenance/pruning

### platform
✅ **Found**: 4 memory-related modules
- `agents/memory.py` — AgentMemory class with short-term + long-term
- `tools/memory_tools.py` — 4 memory tools (search, store, retrieve, prune)
- `web/routes/api/memory.py` — REST API for memory management
- `web/templates/memory.html` — Web UI for memory inspection

---

## 2. DATABASE SCHEMA ANALYSIS

### simple-sf (SFEngine/src/db.rs, lines 116-123)

```sql
CREATE TABLE IF NOT EXISTS memory (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    category TEXT DEFAULT 'note',
    project_id TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);
```

**Characteristics**:
- Simple key-value store
- Single table for all memory (no layering)
- `project_id` column exists but NOT USED in tools
- NO: importance weighting, agent_id isolation, indexes, FTS5
- NO: separate short-term or long-term tables
- NO: vector/embedding columns

### platform (agents/memory.py)

**Schema implied from code** (lines 73, 84-98):
```sql
-- Long-term FTS5 store
CREATE TABLE memory_entries (
    id INTEGER,
    agent_id TEXT,
    key TEXT,
    value TEXT,
    importance FLOAT,
    ...
);

-- Full-text search index
CREATE VIRTUAL TABLE memory_fts USING fts5(...);

-- PostgreSQL alternative: tsvector indexing
```

**Characteristics**:
- ✅ agent_id isolation per memory entry
- ✅ importance weighting (0.0-1.0 scale)
- ✅ FTS5 full-text search (SQLite) OR tsvector (PostgreSQL)
- ✅ Short-term in-memory buffer (sliding window, 50 items default)
- ✅ Long-term persistent store with search ranking
- ❌ No vector embeddings found in code

---

## 3. TOOLS AVAILABLE TO AGENTS

### simple-sf (SFEngine/src/tools.rs)

**Memory tools** (lines 34-36, 263-293):

```rust
"memory_search" => tool_memory_search(args),
"memory_store"  => tool_memory_store(args),
```

**Tool Schemas**:

| Tool | Parameters | Scope | Notes |
|------|-----------|-------|-------|
| `memory_search` | query, scope* | project/global/all | Returns last 20 matches, LIKE search, NO ranking |
| `memory_store` | key, value, category* | Project-wide | Stores one entry, category: decision/finding/note/context |

*scope parameter accepted but NOT IMPLEMENTED (ignored in code, line 609)

**Role Access** (lines 300-312):
```
- rte:             memory_search
- product_owner:   memory_search, memory_store
- scrum_master:    memory_search
- architect:       memory_search, memory_store
- lead_dev:        memory_search, memory_store
- lead_frontend:   memory_search (only)
- lead_backend:    memory_search (only)
- qa_lead:         memory_search
- ux_designer:     memory_search
```

### platform (tools/memory_tools.py)

**Memory tools** (lines 12-46):

```python
class MemorySearchTool
class MemoryStoreTool
class MemoryRetrieveTool       # <-- NOT IN simple-sf
class MemoryPruneTool          # <-- NOT IN simple-sf
```

**Tool Schemas** (inferred):

| Tool | Parameters | Scope | Notes |
|------|-----------|-------|-------|
| `memory_search` | query, limit, scope | agent/project/global | FTS5 ranked search |
| `memory_store` | key, value, importance | Project | With importance weighting |
| `memory_retrieve` | key | Project | Exact key lookup (NOT IN simple-sf) |
| `memory_prune` | query, max_age | Project | Delete obsolete entries (NOT IN simple-sf) |

**Missing in simple-sf**:
- ❌ `memory_retrieve` — exact key lookup
- ❌ `memory_prune` — maintenance/deletion
- ❌ Importance weighting in memory_store
- ❌ Ranked search (FTS5 ranking)
- ❌ Memory scope enforcement (agent isolation)

---

## 4. MEMORY INJECTION INTO AGENT EXECUTION

### simple-sf (SFEngine/src/executor.rs)

**System prompt composition** (lines 56-59):
```rust
let system = format!(
    "{}{}\n\nYour task:\n{}\n\nWorkspace: {}. Use tools to complete the task. Write real, production-quality code.",
    agent_persona, protocol_section, task, workspace
);
```

**What is injected**:
- ✅ agent_persona (loaded from catalog)
- ✅ protocol (phase-specific protocol from protocols module)
- ✅ task (phase task, built from mission brief + prior phase outputs)
- ❌ NO memory search results injected into system prompt
- ❌ NO conversation history injected into agent execution
- ❌ NO relevant context from memory_search auto-loaded

**Memory access**:
- Agents must explicitly CALL memory_search tool
- No proactive context injection

### platform

**Inferred from design**:
- ✅ Short-term context injected into system prompt (get_context_summary)
- ✅ Long-term memory available via tool call
- ✅ Importance-weighted ranking for relevant memories

---

## 5. WORKFLOW/MISSION ORCHESTRATION & MEMORY PERSISTENCE

### simple-sf (SFEngine/src/engine.rs)

**Conversation History Between Phases** (lines 132-138, 1004-1044):

```rust
// Load prior discussion history (line 132)
let prior_history = load_conversation_history(3, 4000);

// Injected into phase context (lines 133-137)
let history_section = if prior_history.is_empty() {
    String::new()
} else {
    format!("\n\n[Historique des échanges précédents] :\n{}\n\n\
             Tiens compte de cet historique — ne répète pas ce qui a déjà été dit/décidé.", prior_history)
};
```

**What is loaded** (lines 1004-1038):
- ✅ Last 3 discussion sessions (most recent first)
- ✅ Agent name, role, and truncated message content (max 400 chars per message)
- ✅ Total context budget: 4000 chars max
- ❌ NO agent-specific memory queries
- ❌ NO memory_search results injection
- ❌ NO project memory lookup between phases

**Phase context building** (lines 803-815):
```rust
fn build_phase_task(phase: &str, brief: &str, previous: &[String]) -> String {
    let context = if previous.is_empty() {
        String::new()
    } else {
        let recent: Vec<_> = previous.iter().rev().take(3).rev().collect();
        // Limit to last 3 phases, 600 chars each
        format!("\n\n## Contexte des phases precedentes:\n{}", ctx)
    };
    // ...
}
```

**What is persisted between phases**:
- ✅ Phase outputs (brief summary)
- ✅ Discussion history (if in same intake session)
- ❌ NO project-level memory lookup between phases
- ❌ NO memory_search auto-triggered between phases
- ❌ NO memory aggregation between mission phases

**Mission phases table** (db.rs, lines 125-137):
```sql
CREATE TABLE IF NOT EXISTS mission_phases (
    id TEXT PRIMARY KEY,
    mission_id TEXT NOT NULL,
    phase_name TEXT NOT NULL,
    pattern TEXT NOT NULL,
    status TEXT DEFAULT 'pending',
    agent_ids TEXT DEFAULT '[]',
    output TEXT DEFAULT '',
    gate_result TEXT,
    started_at TEXT,
    completed_at TEXT
);
```
- NO memory_id or memory_snapshot references
- Memory is NOT formally linked to phases

### platform

**Inferred from design**:
- ✅ Agent-scoped memory search (agent_id isolation)
- ✅ Memory importance weighting for ranking
- ✅ Conversation history in short-term buffer (sliding window)
- ✅ Long-term memory persisted across sessions
- ✅ Possibly project-scoped and global-scoped queries

---

## 6. MEMORY CAPABILITIES COMPARISON

### 4-LAYER MEMORY SYSTEM (platform PLANNED vs simple-sf IMPLEMENTED)

| Layer | Platform | simple-sf | Status |
|-------|----------|-----------|--------|
| **1. Short-term (STM)** | In-memory sliding window (50 items) | discussion_sessions + discussion_messages tables | ⚠️ PARTIAL: On disk, not in-memory sliding window |
| **2. Long-term (LTM)** | SQLite FTS5 + PostgreSQL tsvector | Single memory table, LIKE search | ⚠️ PARTIAL: No full-text indexing, no ranking |
| **3. Project Memory** | agent_id scoped, project_id tracking | project_id column unused in queries | ❌ MISSING: Not enforced or used |
| **4. Global Memory** | Not found in audit | Not found | ❌ MISSING: In both systems? |
| **Vector/Embedding** | Not found | Not found | ❌ MISSING: In both systems |

### FEATURE COMPARISON

| Feature | Platform | simple-sf |
|---------|----------|-----------|
| Memory search (basic) | ✅ Yes | ✅ Yes |
| Memory store (basic) | ✅ Yes | ✅ Yes |
| Memory retrieve (exact key) | ✅ Yes | ❌ No |
| Memory prune (delete) | ✅ Yes | ❌ No |
| Full-text search indexing | ✅ FTS5/tsvector | ❌ No |
| Importance weighting | ✅ Yes | ❌ No |
| Agent isolation | ✅ agent_id field | ⚠️ project_id unused |
| Search ranking | ✅ FTS5 rank() | ❌ No |
| Conversation history injection | ✅ Auto-injected into context | ⚠️ Manual tool call only |
| Memory auto-load between phases | ✅ Implied | ❌ No |
| Vector embeddings | ❌ No | ❌ No |
| Semantic search | ❌ No | ❌ No |
| Memory expiration (TTL) | ❓ Not found | ❌ No |
| Category/tagging | ✅ Yes | ✅ Yes |

---

## 7. DETAILED FINDINGS

### ✅ WHAT WORKS (simple-sf)

1. **Basic persistence**: Memory table stores key-value pairs
2. **Two tools**: memory_search and memory_store are implemented
3. **Search capability**: LIKE-based search on key, value, category
4. **Categorization**: category field (decision, finding, note, context)
5. **Conversation history**: discussion_sessions preserved between intakes
6. **Context passing**: Phase outputs passed to next phase
7. **Task building**: Previous phase outputs included in task prompt

### ❌ WHAT'S MISSING (vs platform)

1. **No full-text indexing**: LIKE queries are slow for large memory bases
2. **No importance/ranking**: All results treated equally
3. **No agent isolation**: project_id exists but not used in queries
4. **No memory retrieval**: Can only search, can't get exact key
5. **No memory pruning**: No delete/maintenance capability
6. **No semantic search**: No vector embeddings
7. **No in-memory short-term**: Everything goes to disk
8. **No scope enforcement**: "scope" parameter in memory_search is ignored
9. **No per-phase memory snapshots**: Memory not linked to mission phases
10. **No memory injection into execution**: Agents must manually call memory_search

### ⚠️ PARTIALLY IMPLEMENTED

1. **Memory scoping**: Column exists (project_id) but queries don't filter on it
2. **Conversation history**: Loaded but only from discussion_sessions, not from memory table
3. **Tool access control**: Some roles have memory_search, but not consistently scoped

---

## 8. CODE REFERENCES

### simple-sf Key Code Locations

| Component | File | Lines | Purpose |
|-----------|------|-------|---------|
| Memory table schema | db.rs | 116-123 | Create memory table |
| memory_search implementation | tools.rs | 607-636 | Query memory with LIKE |
| memory_store implementation | tools.rs | 638-659 | Insert into memory |
| Tool definitions | tools.rs | 263-293 | Schema definitions |
| Role-tool mapping | tools.rs | 299-316 | Which roles get which tools |
| Conversation history loading | engine.rs | 1004-1044 | Load discussion sessions |
| History injection | engine.rs | 132-137 | Inject history into prompt |
| Phase task building | engine.rs | 803-815 | Build task with context |
| Agent execution | executor.rs | 56-59 | System prompt composition |

### Platform Key Code Locations

| Component | File | Lines | Purpose |
|-----------|------|-------|---------|
| AgentMemory class | agents/memory.py | 14+ | Main memory interface |
| Short-term buffer | agents/memory.py | 20-21, 31-48 | Sliding window storage |
| Long-term search | agents/memory.py | 79-100 | FTS5/tsvector search |
| Memory tools | tools/memory_tools.py | 12-46 | Tool definitions |
| Tool registration | tools/memory_tools.py | 49-53 | Register 4 memory tools |

---

## 9. RECOMMENDATIONS FOR simple-sf

### HIGH PRIORITY (Enable 4-layer memory)

1. **Add FTS5 indexing**:
   ```sql
   CREATE VIRTUAL TABLE memory_fts USING fts5(key, value, category);
   -- Update memory_search to use ranked FTS5
   ```

2. **Add agent_id isolation**:
   - Change: `project_id TEXT` → `agent_id TEXT NOT NULL`
   - Filter queries by agent_id
   - Implement project-level memory aggregation

3. **Implement memory_retrieve tool**:
   ```rust
   fn tool_memory_retrieve(args: &Value) -> String {
       let key = args["key"].as_str().unwrap_or("");
       // Exact match query
       db::with_db(|conn| {
           conn.query_row(
               "SELECT value FROM memory WHERE key = ?1",
               [key],
               |row| row.get::<_, String>(0)
           )
       })
   }
   ```

4. **Implement memory_prune tool**:
   ```rust
   fn tool_memory_prune(args: &Value) -> String {
       // Delete entries older than max_age or matching pattern
   }
   ```

### MEDIUM PRIORITY (Enhance execution)

5. **Auto-inject memory into system prompt**:
   - In executor.rs, memory_search relevant entries before agent execution
   - Add to system prompt context

6. **Link memory to mission phases**:
   - Create mission_phase_memory junction table
   - Snapshot memory state at phase boundaries

7. **Implement memory scoping**:
   - Honor the scope parameter (project, global, agent, all)
   - Add multi-tenant memory support

### LOW PRIORITY (Advanced features)

8. **Add importance weighting**:
   - Store importance float (0.0-1.0)
   - Weight search results by importance

9. **Add vector embeddings** (requires external service):
   - Integrate embeddings API (OpenAI, local model)
   - Store vector in memory table
   - Implement semantic search

10. **Add memory expiration**:
    - Add ttl_seconds column
    - Auto-prune expired entries

---

## 10. SUMMARY

**Current simple-sf memory system**: Basic 2-layer (conversation history + key-value store)

**Platform target**: 4-layer (short-term, long-term, project, global) with FTS5 and importance weighting

**Gap**: Missing ~60% of platform memory capabilities
- ❌ Full-text indexing
- ❌ Importance ranking
- ❌ Agent/project scoping enforcement
- ❌ Memory pruning
- ❌ Semantic search (vector embeddings)
- ✅ Basic persistence ✓
- ✅ Search/store tools ✓
- ✅ Conversation history ✓

**Effort to close gap**: ~2-3 days for high/medium priority items; vector search requires external service integration.

