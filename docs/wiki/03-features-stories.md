# Features & User Stories — Simple SF

## FT-SSF-001 — Jarvis AI Chat

**Module:** jarvis | **Layer:** both | **Status:** identified

Interactive chat with context-aware AI assistant that orchestrates multi-agent discussions

### US-SSF-0fde137c: Send message to Jarvis

**As** a developer, **I want** send a message to Jarvis AI, **so that** get an AI-assisted response with agent collaboration

**Priority:** high

- **AC-SSF-028fe221:** GIVEN the chat is open WHEN I type a message and press send THEN Jarvis processes my message and returns a response within the chat
- **AC-SSF-4d0cf184:** GIVEN Jarvis receives my message WHEN it requires multi-agent input THEN a network discussion is launched with relevant agents
- **AC-SSF-dcf54c17:** GIVEN agents are discussing WHEN each agent contributes THEN their response appears with avatar, role badge, and colored card

### US-SSF-b630e46b: View agent discussion

**As** a tech lead, **I want** see all agent contributions in a discussion, **so that** understand the multi-perspective analysis

**Priority:** high

- **AC-SSF-6be98ced:** GIVEN a discussion is active WHEN agents contribute THEN each message shows agent name, role, avatar, and timestamp
- **AC-SSF-aa1124aa:** GIVEN the discussion completes WHEN I review results THEN I see a structured summary with all contributions

### US-SSF-dc5739c8: Stream responses

**As** a developer, **I want** see responses streaming in real-time, **so that** not wait for full completion

**Priority:** medium

- **AC-SSF-0409c0b7:** GIVEN I send a message WHEN the LLM starts responding THEN text appears token by token in the chat bubble
- **AC-SSF-f0fa50d8:** GIVEN streaming is active WHEN there's a reasoning phase THEN I see a thinking indicator before content appears

## FT-SSF-002 — Multi-Agent Discussions

**Module:** agents | **Layer:** both | **Status:** identified

22+ agents with real personas collaborate in network/sequential/parallel patterns

### US-SSF-845cc996: Launch network discussion

**As** a tech lead, **I want** start a multi-agent network discussion, **so that** get diverse expert perspectives on a topic

**Priority:** high

- **AC-SSF-4d15f40b:** GIVEN I have a topic requiring multiple viewpoints WHEN I launch a network discussion THEN agents with relevant roles are selected and begin discussing
- **AC-SSF-6ac35a52:** GIVEN the discussion is running WHEN agents interact THEN each agent sees previous contributions and builds upon them

### US-SSF-dd0485ce: Agent role diversity

**As** a developer, **I want** see agents with distinct roles and expertise, **so that** get specialized input from each role

**Priority:** high

- **AC-SSF-a1387271:** GIVEN a discussion includes multiple agents WHEN each agent contributes THEN their response reflects their specific role and expertise
- **AC-SSF-947e8149:** GIVEN I view agent profiles WHEN I check their details THEN I see persona, system prompt, role, and tagline

## FT-SSF-003 — Project Management

**Module:** projects | **Layer:** both | **Status:** identified

Create, list, update, delete projects with tech stack and status tracking

### US-SSF-3bcc856d: Create project

**As** a developer, **I want** create a new project with name, description, and tech stack, **so that** organize my work

**Priority:** high

- **AC-SSF-8ba71644:** GIVEN I'm on the projects page WHEN I fill in project details and submit THEN a new project is created and appears in the list
- **AC-SSF-e69270ba:** GIVEN I create a project WHEN it's saved THEN the project has a unique ID, timestamps, and 'idea' status

### US-SSF-43ae4b3e: List and manage projects

**As** a developer, **I want** see all my projects and manage them, **so that** track project status

**Priority:** high

- **AC-SSF-8ebf3bec:** GIVEN I have projects WHEN I view the projects list THEN all projects are shown with name, tech, status, and dates
- **AC-SSF-0b034a9c:** GIVEN I select a project WHEN I choose to delete it THEN the project is removed after confirmation

## FT-SSF-004 — Mission Orchestration

**Module:** missions | **Layer:** both | **Status:** identified

Launch and monitor multi-phase missions with workflow patterns

### US-SSF-6bed27b3: Start mission

**As** a developer, **I want** launch a mission with a brief, **so that** have agents execute the task autonomously

**Priority:** high

- **AC-SSF-ba4bf50f:** GIVEN I have a project WHEN I start a mission with a brief THEN agents are assigned and phases begin executing
- **AC-SSF-ed7dbfe9:** GIVEN a mission is running WHEN I check status THEN I see current phase, agent activity, and progress

### US-SSF-899f7f2e: Monitor mission progress

**As** a tech lead, **I want** monitor real-time mission progress, **so that** intervene if needed

**Priority:** medium

- **AC-SSF-4e412319:** GIVEN a mission is active WHEN I view the mission dashboard THEN I see phase progression, agent outputs, and guard results
- **AC-SSF-9f7aca4e:** GIVEN a phase fails guard checks WHEN the system detects issues THEN the mission retries or escalates per PUA rules

## FT-SSF-005 — LLM Provider Management

**Module:** llm | **Layer:** both | **Status:** identified

10 LLM providers: Ollama, MLX, OpenAI, Anthropic, Gemini, MiniMax, Kimi, OpenRouter, Qwen, Zhipu

### US-SSF-ac1e5a6f: Configure LLM provider

**As** a developer, **I want** set up an LLM provider with API key, **so that** use AI features

**Priority:** critical

- **AC-SSF-a53141e7:** GIVEN I'm in settings WHEN I select a provider and enter credentials THEN the provider is configured and ready for use
- **AC-SSF-8beedd90:** GIVEN I configure Ollama locally WHEN I test the connection THEN I see available models and can select one

### US-SSF-ec901d79: Switch providers

**As** a developer, **I want** switch between LLM providers, **so that** use the best model for each task

**Priority:** medium

- **AC-SSF-22a76271:** GIVEN multiple providers are configured WHEN I switch to a different provider THEN the change takes effect immediately for new requests
- **AC-SSF-6b99e9f3:** GIVEN I switch providers WHEN an ongoing chat continues THEN existing messages are preserved and new ones use the new provider

## FT-SSF-006 — Ideation Engine

**Module:** ideation | **Layer:** both | **Status:** identified

AI-powered brainstorming with structured output and agent collaboration

### US-SSF-46fae0ad: Start ideation session

**As** a product owner, **I want** brainstorm ideas with AI agents, **so that** explore product directions

**Priority:** high

- **AC-SSF-75d526fb:** GIVEN I'm on the ideation page WHEN I enter an idea topic THEN an ideation session starts with multiple agents contributing
- **AC-SSF-db8ecd2a:** GIVEN agents brainstorm WHEN the session progresses THEN I see structured output with categories and actionable items

## FT-SSF-007 — Onboarding & Setup

**Module:** onboarding | **Layer:** swift | **Status:** identified

First-run wizard for LLM provider setup and configuration

### US-SSF-f814343b: Complete first-run setup

**As** a new user, **I want** complete the setup wizard, **so that** start using the app

**Priority:** critical

- **AC-SSF-57a7c8fd:** GIVEN I launch the app for the first time WHEN the onboarding screen appears THEN I see a welcome message and setup steps
- **AC-SSF-b8ec861d:** GIVEN I'm in the wizard WHEN I configure an LLM provider THEN I can test the connection and proceed to the main app

## FT-SSF-008 — Rich Markdown Rendering

**Module:** views | **Layer:** swift | **Status:** identified

Native SwiftUI markdown: headers, bold, lists, tables, code blocks, blockquotes

### US-SSF-0de8865d: Render rich markdown

**As** a developer, **I want** see properly formatted markdown in chat, **so that** read agent output easily

**Priority:** high

- **AC-SSF-7ce0e088:** GIVEN an agent returns markdown content WHEN I view the message THEN headers, bold, lists, tables, and code blocks render correctly
- **AC-SSF-4a6d6319:** GIVEN markdown contains a code block WHEN I view it THEN syntax highlighting is applied with monospace font

## FT-SSF-009 — Chat History & Persistence

**Module:** data | **Layer:** swift | **Status:** identified

JSON-based session persistence across restarts with full agent metadata

### US-SSF-87ea0424: Persist chat history

**As** a developer, **I want** have my chat sessions saved, **so that** continue conversations later

**Priority:** high

- **AC-SSF-1c0d8431:** GIVEN I have active chat sessions WHEN I quit and relaunch the app THEN all previous sessions and messages are restored
- **AC-SSF-9d297823:** GIVEN I load history WHEN the data includes agent metadata THEN avatars, roles, and timestamps are all preserved

## FT-SSF-010 — Agent Catalog

**Module:** agents | **Layer:** both | **Status:** identified

185 agents with avatars, roles, personas, system prompts, hierarchy

### US-SSF-171bb64a: Browse agent catalog

**As** a developer, **I want** browse all available agents, **so that** understand team composition

**Priority:** medium

- **AC-SSF-2b9578a5:** GIVEN I open the agents view WHEN I browse the catalog THEN I see all 185 agents with avatars, names, roles, and taglines
- **AC-SSF-00694024:** GIVEN I search for an agent role WHEN I filter the list THEN only agents matching the role criteria are shown

## FT-SSF-011 — Adversarial Guard L0

**Module:** guard | **Layer:** rust | **Status:** identified

Deterministic quality checks: slop, mock, fake build, hallucination detection

### US-SSF-35555106: Detect slop and fake output

**As** a QA engineer, **I want** have agent output automatically checked for quality, **so that** ensure no fake or low-quality code passes

**Priority:** critical

- **AC-SSF-b2b16789:** GIVEN an agent produces output WHEN the L0 guard runs THEN slop, mock data, fake builds, and hallucinations are detected
- **AC-SSF-6dd24361:** GIVEN guard detects issues (score >= 7) WHEN the output is rejected THEN the agent must retry with corrected output
- **AC-SSF-4a640a60:** GIVEN guard passes (score < 5) WHEN the output is accepted THEN it proceeds to the next phase

## FT-SSF-012 — Code Sandbox

**Module:** sandbox | **Layer:** rust | **Status:** identified

Secure execution sandbox for agent-generated code

### US-SSF-804e5a15: Execute code safely

**As** a developer, **I want** run agent-generated code in a sandbox, **so that** prevent harm to my system

**Priority:** critical

- **AC-SSF-a796b375:** GIVEN an agent generates executable code WHEN the sandbox is invoked THEN code runs in an isolated environment with limited permissions
- **AC-SSF-eca380db:** GIVEN sandboxed code tries to access restricted resources WHEN the sandbox blocks it THEN an error is returned without system impact

## FT-SSF-013 — Design System

**Module:** ui | **Layer:** swift | **Status:** identified

Adaptive light/dark theme with design tokens, colors, typography, spacing

### US-SSF-7bc62242: Consistent visual design

**As** a developer, **I want** have a consistent look across all views, **so that** have a professional user experience

**Priority:** high

- **AC-SSF-6cf95c44:** GIVEN I use the app in dark mode WHEN I navigate between views THEN all screens use consistent colors, fonts, spacing from design tokens
- **AC-SSF-90c2e526:** GIVEN I switch to light mode WHEN the app adapts THEN all colors properly adjust for readability in light appearance

## FT-SSF-014 — Agent Avatars

**Module:** ui | **Layer:** swift | **Status:** identified

22 photo avatars with fallback initials, cached loading

### US-SSF-31ee5509: Display agent avatars

**As** a developer, **I want** see photo avatars for agents, **so that** quickly identify who is speaking

**Priority:** medium

- **AC-SSF-b504e32e:** GIVEN a chat includes agents WHEN their messages appear THEN each message shows the agent's photo avatar
- **AC-SSF-0c30539f:** GIVEN an avatar image is missing WHEN the fallback activates THEN initials are shown in a styled circle

## FT-SSF-015 — i18n Localization

**Module:** i18n | **Layer:** swift | **Status:** identified

12 languages via Localizable.xcstrings

### US-SSF-147550ff: Use app in multiple languages

**As** a developer, **I want** use the app in my preferred language, **so that** work comfortably

**Priority:** medium

- **AC-SSF-2269d3f1:** GIVEN I set my system language to French WHEN I open the app THEN all UI text appears in French
- **AC-SSF-f93d5d37:** GIVEN the app supports 12 languages WHEN I switch language THEN all labels, buttons, and messages update

## FT-SSF-016 — Git Push Export

**Module:** output | **Layer:** swift | **Status:** identified

Push agent-generated code to Git repositories

### US-SSF-0b5e0c1e: Push code to Git

**As** a developer, **I want** push agent-generated code to a Git repository, **so that** share and version my work

**Priority:** high

- **AC-SSF-e0dc03a3:** GIVEN a mission generates code files WHEN I trigger git push THEN code is committed and pushed to the configured repository

## FT-SSF-017 — Zip Export

**Module:** output | **Layer:** swift | **Status:** identified

Export project workspace as ZIP archive

### US-SSF-7ce1cccd: Export workspace as ZIP

**As** a developer, **I want** export my workspace as a ZIP file, **so that** share or archive my work

**Priority:** medium

- **AC-SSF-4a2f8322:** GIVEN I have workspace files WHEN I trigger ZIP export THEN a ZIP archive is created with all workspace contents

## FT-SSF-018 — SQLite Database

**Module:** db | **Layer:** rust | **Status:** identified

WAL-mode SQLite with full schema: projects, missions, agents, discussions, tools

### US-SSF-75b8401d: Reliable data persistence

**As** a developer, **I want** have all data reliably persisted, **so that** not lose any work

**Priority:** critical

- **AC-SSF-241806ac:** GIVEN the app is running WHEN data is written THEN SQLite WAL mode ensures crash-safe writes
- **AC-SSF-6de83ef6:** GIVEN the app starts WHEN the schema is initialized THEN all required tables are created with proper constraints

## FT-SSF-019 — Tool Execution

**Module:** tools | **Layer:** rust | **Status:** identified

Agent tool calls: code_write, shell_exec, file_read, web_search, etc.

### US-SSF-63265f6b: Agent tool calls

**As** a developer, **I want** have agents call tools to perform actions, **so that** get real work done

**Priority:** high

- **AC-SSF-852cf7df:** GIVEN an agent needs to write code WHEN it calls code_write tool THEN the file is created in the workspace
- **AC-SSF-a443ba3e:** GIVEN an agent needs to execute a command WHEN it calls shell_exec tool THEN the command runs and output is captured

## FT-SSF-020 — Workflow Engine

**Module:** engine | **Layer:** rust | **Status:** identified

YAML-defined workflows with phase templates and pattern orchestration

### US-SSF-84fa3ae0: Execute workflow phases

**As** a tech lead, **I want** have missions follow defined workflow phases, **so that** ensure structured execution

**Priority:** high

- **AC-SSF-5a8c2a22:** GIVEN a mission uses a workflow WHEN phases execute in order THEN each phase uses the correct pattern and team
- **AC-SSF-82e33f22:** GIVEN a phase requires specific agents WHEN the engine selects them THEN agents matching required roles are assigned

## FT-SSF-021 — Benchmark Suite

**Module:** bench | **Layer:** rust | **Status:** identified

Performance benchmarking for agent execution and LLM calls

### US-SSF-dce5ce1a: Run benchmarks

**As** a QA engineer, **I want** run performance benchmarks, **so that** measure agent and LLM performance

**Priority:** low

- **AC-SSF-1078a550:** GIVEN I trigger benchmarks WHEN tests execute THEN I see timing results for agent execution and LLM calls

## FT-SSF-022 — Code Indexer

**Module:** indexer | **Layer:** rust | **Status:** identified

Tree-sitter based code indexing for 7 languages

### US-SSF-e44f0967: Index codebase

**As** a developer, **I want** have the codebase indexed for agent context, **so that** give agents better code understanding

**Priority:** medium

- **AC-SSF-ab60b047:** GIVEN a project has source code WHEN the indexer runs THEN tree-sitter parses files in 7 languages and creates an index

## FT-SSF-023 — REST API Server

**Module:** server | **Layer:** rust | **Status:** identified

Axum-based HTTP API with JWT auth, CRUD endpoints, SSE streaming

### US-SSF-9676abc8: Access API endpoints

**As** a developer, **I want** use REST API endpoints, **so that** integrate with external tools

**Priority:** medium

- **AC-SSF-939a5391:** GIVEN the server is running WHEN I call /health THEN I get a 200 OK with version info
- **AC-SSF-bd67ff69:** GIVEN I'm authenticated WHEN I call CRUD endpoints THEN I can create, read, update, delete resources

## FT-SSF-024 — Authentication

**Module:** auth | **Layer:** rust | **Status:** identified

JWT-based auth with login, register, user management

### US-SSF-eeada6fd: Register and login

**As** a developer, **I want** register and authenticate, **so that** access secured features

**Priority:** high

- **AC-SSF-e728bfb5:** GIVEN I'm on the login page WHEN I register with email and password THEN my account is created and I'm authenticated
- **AC-SSF-e5e3b63d:** GIVEN I'm registered WHEN I login THEN I receive a JWT token for subsequent requests

## FT-SSF-025 — Evaluation Engine

**Module:** eval | **Layer:** rust | **Status:** identified

Agent output evaluation with scoring and quality metrics

### US-SSF-cd35155f: Evaluate agent output

**As** a QA engineer, **I want** have agent output evaluated for quality, **so that** ensure consistent quality

**Priority:** medium

- **AC-SSF-3cfff8d4:** GIVEN an agent produces output WHEN the evaluator scores it THEN I see quality metrics and a pass/fail decision

