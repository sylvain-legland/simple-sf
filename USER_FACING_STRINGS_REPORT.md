# SimpleSF macOS App - User-Facing Strings Extraction

**Location:** `/Users/sylvain/_MACARON-SOFTWARE/simple-sf/SimpleSF/`

**Status:** Complete file review of all requested directories

---

## Existing i18n Infrastructure (Localizable.xcstrings)

**File:** `SimpleSF/i18n/Localizable.xcstrings`

**Format:** 
- JSON-based Apple Localizable Strings catalog
- SourceLanguage: English ("en")
- 30+ supported languages (af, ar, bn, ca, cs, da, de, el, en, es, fi, fr, he, hi, hr, hu, id, it, ja, ko, lt, lv, ms, nb, nl, pl, pt, ro, ru, sk, sl, sr, sv, th, tl, tr, uk, ur, vi, zh)
- Hierarchical key structure: `category.key`
- Each string has full translations per language

**Currently Localized Keys:**
- `app.name` - "Simple SF"
- `nav.projects`, `nav.jarvis`, `nav.ideation` - Navigation menu items
- `mode.simple`, `mode.advanced` - Mode selections
- `action.start`, `action.pause`, `action.stop` - Action buttons
- `jarvis.welcome` - "Hi, I'm Jarvis"
- `onboarding.title` - "Configure your LLM providers"
- `output.export_zip` - "Export as ZIP"
- `output.push_git` - "Push to Git"

---

## User-Facing Strings by File

### 1. SimpleSF/App/SimpleSFApp.swift
**No user-facing strings** (initialization & setup only)

### 2. SimpleSF/App/AppState.swift
**No user-facing strings** (state management only)

### 3. SimpleSF/Views/Shared/ContentView.swift
**No user-facing strings** (routing view only)

### 4. SimpleSF/Views/Shared/MainView.swift
```
NAVIGATION (Sidebar):
  Labels:
    - "Jarvis"
    - "Projects"
    - "Ideation"
    - "Settings"
    
HEADER BANNER:
  - "Software Factory" (main app title with hammer icon)
  
MODE TOGGLE:
  Labels:
    - "YOLO" (auto-approve gate mode)
  Help:
    - "YOLO: auto-approve all GO/NOGO gates"

PROVIDER BADGE (Bottom):
  Labels:
    - "No LLM" (when no provider configured)
    - "Not configured" (model fallback)
    - "loading…" (loading state)
  Buttons:
    - "Auto" (switch to auto-detect) - Tooltip: "Switch back to auto-detect"
```

### 5. SimpleSF/Views/Shared/MarkdownView.swift
**No user-facing strings** (pure markdown rendering engine)

### 6. SimpleSF/Views/Mission/MissionView.swift (KEY - 681 lines)
```
EMPTY STATE (When no mission running):
  Title:
    - "Value Stream"
  CTA:
    - "Lancer un Epic" (Launch Epic)
  Description:
    - "Décrivez votre produit. L'équipe SAFe enchaîne 14 phases :\n
       Idéation → Comité Stratégique → Architecture → Sprints Dev → QA → 
       Deploy Prod → TMA"
  Form Elements:
    - "Sélectionner un projet…" (project picker placeholder)
    - "Lancer le Workflow SAFe" (launch button)

EPIC HEADER (When mission active):
  - "Cycle de Vie Produit Complet" (Full Product Lifecycle)

MISSION STATUS BADGES:
  - "En cours" (Running)
  - "Terminé" (Completed)
  - "Échoué" (Failed)
  - "Véto" (Vetoed/Blocked)

PHASE TIMELINE/DETAILS:
  Status Messages:
    - "Phase en attente" (Phase pending)
    - "Discussion en cours…" (Discussion ongoing)
  Result Label:
    - "Résultat de phase" (Phase result)
  
TIME DISPLAY:
  - Timestamps shown in HH:MM:SS format
```

### 7. SimpleSF/Views/Agents/AgentsView.swift
```
HEADER:
  - "Agent Team" (with person.3.fill icon)
  
AGENT COUNT:
  - "[count] agents" (badge)

AGENT CARDS:
  Each card shows:
    - [Agent name]
    - [Agent role - dynamically rendered]
    - [Agent persona - truncated to 3 lines]
```

### 8. SimpleSF/Jarvis/JarvisView.swift (KEY - 27.3 KB)
```
HEADER:
  - "Jarvis" (main title)

STATUS INDICATOR:
  Messages:
    - "Réflexion en cours…" (Thinking...)
    - "L'equipe se reunit..." (Team gathering...)

SESSION SIDEBAR:
  Section Title:
    - "Historique" (History)
  Ready State:
    - "Votre equipe est prete" (Your team is ready)
  Capability Summary:
    - "192 agents · 1286 skills · 19 patterns"
  Suggestion:
    - "Essayez : « Fais-moi un Pacman en SwiftUI »"
       (Try: "Make me a Pac-Man in SwiftUI")
  Session Items:
    - [Session title]
    - "[count] messages" (message count per session)

MESSAGE DISPLAY:
  - Rendered messages from agent or user
  - Markdown support for formatting
```

### 9. SimpleSF/Projects/ProjectsView.swift (KEY - 73.1 KB)
```
HEADER:
  - "Projects"
  - "[count] projects" (count badge)

SEARCH:
  - "Search projects…" (text field placeholder)

EMPTY STATE:
  - "No projects yet"
  - "Ask Jarvis to create a project for you.\n
     \"Create a project called MyApp using Swift\""
     (instructional empty state)

PILOT PROJECTS SECTION:
  Section Title:
    - "Projets Pilotes" (Pilot Projects)
  Subtitle:
    - "AC validation"
  Buttons:
    - "Charger" (Load)
    - "Réinitialiser" (Reset)
  Help Text:
    - "Cliquez \"Charger\" pour importer les 8 projets pilotes"
      (Click Load to import 8 pilot projects)

PILOT PROJECT CARDS:
  [8 predefined projects with tech stacks and descriptions in French]

PROJECT ACCORDION:
  - "Cycle de Vie Produit Complet" (Full Product Lifecycle)
  - "[phase_count] phases"
  - "Workflow non défini — lancez le projet pour que le PM décide du cycle de vie"
    (Workflow undefined - launch to let PM decide)

PHASE TIMELINE:
  Phase Indicators:
    - "1", "2", "3", ... (phase numbers)
  Phase Status:
    - "Agents en cours…" (Agents working)
    - "Prêt" (Ready)
    - "Queued"
    - "Paused"
    - "Terminé" (Completed)

DISCUSSION MESSAGES:
  For each message:
    - [Agent name]
    - [Agent role with color]
    - [Timestamp HH:MM:SS]
    - [Message content - truncated to 500 chars in preview]
  Result Display:
    - "Résultat de phase" (Phase result header)
  Count:
    - "[count] messages"
```

### 10. SimpleSF/Ideation/IdeationView.swift
```
HEADER:
  - "Ideation" (with lightbulb.fill icon)

RUNNING STATE:
  - "Agents discussing..." (status while running)

INPUT SECTION:
  Label:
    - "Describe your idea"
  Form:
    - Text editor for idea input
  Error Display:
    - [Error message in red if present]

BUTTON:
  - "Launch Ideation" (main CTA)

PROVIDER CHECK:
  If no provider:
    - "Configure a provider in Settings" (with warning icon)
  If provider available:
    - "3 agents" (badge)
    - "3 rounds" (badge)
    - "network pattern" (badge)

COMPLETION STATE:
  - "Ideation complete — 3 perspectives, 3 rounds" (centered message)
```

### 11. SimpleSF/Onboarding/SetupWizardView.swift (KEY - 22.8 KB)
```
PROGRESS INDICATOR:
  - 4 progress dots (filled based on step)

=== STEP 1: WELCOME ===
Title:
  - "Simple SF"
Subtitle:
  - "Your private Software Factory\n
     Powered by AI agents, running 100% on your Mac."
System Info:
  - "Apple [CHIP_NAME] · [RAM_GB] GB unified memory"
Buttons:
  - "Get Started" (CTA)
  - "Skip — I'll configure later" (secondary)

=== STEP 2: CHOOSE ENGINE ===
Title:
  - "How do you want to run AI?"
Description:
  - "Everything stays on your Mac. No data leaves your machine."
Engine Options:
  - MLX Card (apple.logo icon)
  - Ollama Card (desktopcomputer icon)
Buttons:
  - "Skip — use cloud API keys instead"
  - "Back"

=== STEP 3: MLX MODEL SELECTION ===
Title:
  - "Choose your model"
System Memory:
  - "[RAM_GB] GB unified memory — recommended:"
  - "[MODEL_NAME]"
Model List:
  For each model:
    - [Model name]
    - "Recommended" (badge)
    - "Installed" (badge)
    - "[params] · [quant] · [size_GB] GB · min [min_RAM] GB RAM"
  Button per model:
    - "Use this model"
    - "Download [size_GB] GB"
Navigation:
  - "Back"

=== STEP 4: DOWNLOADING ===
Status:
  - "Downloading [model_name]..."
  - "[size_GB] GB from HuggingFace"
  - Progress indicator with log output
Buttons:
  - "Cancel"

Download Completion:
  - "Download complete!"
  - [Model name]
  Button:
    - "Continue"

Download Failure:
  - "Download failed"
  - [Error message]
  Buttons:
    - "Retry"
    - "Back"

=== STEP 5: OLLAMA SETUP ===
Title:
  - "Ollama Setup"

When Running:
  - "Ollama is running" (with checkmark icon)
  - "Available models:"
  - [Model list with names and sizes]
  Button:
    - "Continue with Ollama"

When No Models:
  - "No models installed. Run in Terminal:"
  - "ollama pull qwen3:14b"
  Button:
    - "Refresh"

When Not Running:
  - "Ollama is not running"
  Buttons:
    - "Open Ollama.app"
    - "Install Ollama"
    - "Refresh"
  - "Back"

=== STEP 6: COMPLETION ===
Title:
  - "You're all set!"
Description:
  - "Your private AI Software Factory is ready.\n
     Talk to Jarvis to start building."
Configuration Display:
  - "MLX · [model_name]" (if MLX selected)
  - "Ollama · [model_name]" (if Ollama selected)
Button:
  - "Start using Simple SF"
```

### 12. SimpleSF/Onboarding/OnboardingView.swift (KEY - 26.3 KB)
```
HEADER:
  - "Settings" (with gearshape.fill icon)

SECURITY NOTE SECTION:
  - "Local models run 100% on your Mac. No data leaves your machine."
  - "Cloud API keys stored in macOS Keychain — sent only to the provider."

=== ACTIVE MODEL BANNER ===
When Provider Active:
  - "[Provider displayName]"
  - [Active model name/short name]
When No Provider:
  - "No model active"
  - "Select a provider below"
Help Text:
  - "Configure a local or cloud provider below, then click \"Use\" to activate it."
Button (when selected):
  - "Auto" (with arrow.counterclockwise icon) - Tooltip: "Switch back to auto-detect"

=== LANGUAGE SECTION ===
Label:
  - "Language"
Dropdown:
  - [30+ language options with native names]
Help:
  - "Jarvis responds in this language"

=== LOCAL MODELS SECTION ===
Header:
  - "Local Models"
Subtitle:
  - "100% on-device · no internet needed"

--- OLLAMA PROVIDER ---
Name:
  - "Ollama"
Subtitle:
  - [From LLMProvider.ollama.subtitle]
Status:
  - "Running" (circle.fill icon) OR "Stopped" (circle icon)
Use Button:
  - "Use" (when stopped)
  - "Active" badge (when active)
When Running:
  - "Model:" label
  - [Model name list]
  - "[model_name] ([size])"
  Buttons:
    - "Stop"
When Empty:
  - "No models installed. Run: ollama pull qwen3:14b"
When Stopped:
  - "Port [port]"
  Buttons:
    - "Start Ollama"
  Alternative:
    - "or run: ollama serve"
Refresh Button:
  - "Refresh"

--- APPLE MLX PROVIDER ---
Name:
  - "Apple MLX"
Subtitle:
  - [From LLMProvider.mlx.subtitle]
Use Button:
  - "Use" (or "Active" badge when active)
When Running:
  - "Model:" label
  - [Model list with details]
  - "[model_name]"
  - "[model_type]" (e.g., LoRA)
  - "[size] GB"
  Buttons:
    - "Stop"
When Empty:
  - "No MLX models in ~/.cache/huggingface/hub/ or ~/.cache/mlx-models/"
Toggle:
  - "Auto-start" (for MLX server)
When Stopped:
  - "Port [port]"
  Buttons:
    - "Start Server"
Scan Button:
  - "Refresh" (to scan models)

=== CLOUD PROVIDERS SECTION ===
Header:
  - "Cloud Providers"
Subtitle:
  - "Requires API key"

Provider Card (Expandable):
  Header:
    - [Provider displayName]
    - [Provider subtitle]
  When Collapsed:
    - "Use" button
    - "Active" badge (when active)
    - Expand button
  When Expanded:
    - Model field:
      - "Model:" label
      - TextField for model override
    - "Test" button
    - "Save" button
    - "Delete" button (destructive)
    - "Active" badge (when active)
    - "Use" button (when not active)
  - "Get API key →" (link button at bottom)

Status Indicators:
  - "Active" (checkmark.circle.fill)
  - "Starting..." (while loading)
  - "Stopped" (circle)
  - [Error message with exclamationmark.circle.fill]
```

### 13. SimpleSF/Output/ZipExporter.swift
**No user-facing strings in UI**

Error Messages (LocalizedError):
- "Workspace not found: [path]"
- "Export cancelled"
- "zip failed: [output]"

UI String:
- `$0.prompt = "Export"` (file save dialog)

### 14. SimpleSF/Output/GitPusher.swift
**No user-facing strings in UI**

Error Messages (LocalizedError):
- "Project workspace not found"
- "git [cmd] failed:\n[output]"

Git Configuration:
- `commitMessage: String = "feat: update from Simple SF"` (default commit message)

### 15. SimpleSF/Views/Shared/DesignTokens.swift
**No user-facing strings** (design tokens & components only)

Helper component displays:
- Initials circle: "?" (fallback when no initials available)
- Role badge: `role.uppercased()`
- Pattern badge: `pattern` (network, sequential, parallel, hierarchical, loop)

---

## String Categories Summary

### NAVIGATION/TITLES
- "Software Factory", "Jarvis", "Projects", "Ideation", "Settings", "Value Stream", "Ideation", "Agent Team"

### BUTTONS & ACTIONS
**Primary Actions:**
- "Launch Ideation", "Launch Epic", "Lancer le Workflow SAFe"
- "Get Started", "Start using Simple SF", "Continue"

**Secondary Actions:**
- "Use", "Auto", "Active", "Skip", "Back", "Cancel", "Refresh"
- "Charger" (Load), "Réinitialiser" (Reset)
- "Stop", "Start", "Start Server", "Start Ollama"
- "Open Ollama.app", "Install Ollama"
- "Test", "Save", "Delete", "Get API key"

**In i18n:**
- action.start (Start), action.pause (Pause), action.stop (Stop)
- output.export_zip, output.push_git

### LABELS & DESCRIPTIONS
- "Describe your idea", "Choose your model", "How do you want to run AI?"
- "Language", "Local Models", "Cloud Providers", "Ollama", "Apple MLX"
- "Available models", "Model:", "Cycle de Vie Produit Complet"

### STATUS/STATE MESSAGES
- Running: "Réflexion en cours…", "L'equipe se reunit...", "Agents discussing...", "Agents working"
- Completed: "Terminé", "You're all set!", "Download complete!"
- Error: "Failed", "Échoué", "Blocked" (Véto), "Download failed"
- Model States: "No LLM", "Not configured", "loading…", "Active", "Stopped", "Running", "Starting..."
- Ready: "Prêt", "Ready"
- Pending: "Queued", "Paused", "Phase pending"

### EMPTY STATES
- "No projects yet" + "Ask Jarvis to create a project for you..."
- "No models installed. Run: ollama pull qwen3:14b"
- "Ollama is not running"
- "No MLX models in ~/.cache/huggingface/hub/ or ~/.cache/mlx-models/"

### HELP TEXT & PROMPTS
- "Everything stays on your Mac. No data leaves your machine."
- "Local models run 100% on your Mac. No data leaves your machine."
- "Cloud API keys stored in macOS Keychain — sent only to the provider."
- "Configure a local or cloud provider below, then click \"Use\" to activate it."
- "Jarvis responds in this language"
- "100% on-device · no internet needed"

### DYNAMIC/RUNTIME STRINGS
- "[count] projects", "[count] agents", "[count] messages"
- "[phase_count] phases", "[model_name]", "[size] GB", "[RAM] GB"
- "192 agents · 1286 skills · 19 patterns"
- Timestamps in HH:MM:SS format

### SYSTEM INFO
- "Apple [CHIP_NAME] · [RAM_GB] GB unified memory"
- "Port [port]"

### FRENCH-SPECIFIC STRINGS
⚠️ **NOTE:** Many UI strings are in FRENCH, suggesting app is designed for French-speaking users:
- "Lancer un Epic", "Décrivez votre produit", "Réflexion en cours…"
- "Terminé", "Échoué", "Véto", "Phase en attente", "Discussion en cours…"
- "Cycle de Vie Produit Complet", "Workflow non défini"
- "Projets Pilotes", "Charger", "Réinitialiser"
- "Reunion de cadrage", "En cours", "Historique"
- "Votre equipe est prete", "Agents discussing..."

---

## Localization Strategy Recommendations

### Needed i18n Keys to Add:
Based on code review, these hardcoded strings should be moved to i18n:

**Navigation/UI:**
- "nav.settings" (currently hardcoded "Settings")
- "header.software_factory"
- "toggle.yolo" + help text

**Mission/Value Stream:**
- "mission.value_stream"
- "mission.launch_epic"
- "mission.launch_workflow_safe"
- "mission.describe_product"
- "mission.select_project"
- "mission.lifecycle_complete"
- "mission.status.*" (completed, failed, vetoed, running, pending)
- "mission.phase_pending", "mission.discussion_ongoing", "mission.phase_result"

**Projects:**
- "projects.empty_state_header"
- "projects.empty_state_help"
- "projects.pilot_projects"
- "projects.ac_validation"
- "projects.load_pilots"
- "projects.import_message"
- "projects.phases"
- "projects.workflow_undefined"

**Ideation:**
- "ideation.describe_idea"
- "ideation.launch"
- "ideation.agents_discussing"
- "ideation.configure_provider"
- "ideation.rounds"
- "ideation.complete"

**Onboarding:**
- Multiple setup step strings (welcome, choose engine, choose model, etc.)

**Settings:**
- "settings.language"
- "settings.jarvis_language"
- "settings.local_models"
- "settings.cloud_providers"
- "settings.ollama", "settings.mlx"
- "settings.model_select", "settings.auto_start"
- Error/status messages

**Output:**
- Error messages from ZipExporter and GitPusher

### Implementation:
1. Extract remaining hardcoded strings to string keys
2. Use `String(localized: "key.name")` or `NSLocalizedString()`
3. Update Localizable.xcstrings with new keys
4. Run Xcode's "Export Localizations" for translator review
5. Consider using `Locale` for language selection (already in AppState.selectedLang)

---

## File Path Reference

All paths relative to `/Users/sylvain/_MACARON-SOFTWARE/simple-sf/SimpleSF/`:

```
App/
  ├── SimpleSFApp.swift
  └── AppState.swift

Views/
  ├── Shared/
  │   ├── ContentView.swift
  │   ├── MainView.swift (SidebarItem enum, SidebarView)
  │   ├── MarkdownView.swift
  │   └── DesignTokens.swift
  ├── Mission/
  │   └── MissionView.swift
  └── Agents/
      └── AgentsView.swift

Jarvis/
  └── JarvisView.swift

Projects/
  └── ProjectsView.swift

Ideation/
  └── IdeationView.swift

Onboarding/
  ├── SetupWizardView.swift
  └── OnboardingView.swift

Output/
  ├── ZipExporter.swift
  └── GitPusher.swift

i18n/
  └── Localizable.xcstrings (existing translations for 30+ languages)
```

---

## Key Statistics

- **Total Files Reviewed:** 15 Swift files + 1 i18n file
- **Files with Hardcoded Strings:** 12
- **Estimated Hardcoded User-Facing Strings:** 150+
- **Already Localized (i18n):** 10 keys with 30+ language translations
- **Strings Needing Localization:** 140+ (estimated)
- **Languages Supported:** 30+ (English as source, all in Localizable.xcstrings)
- **Localization Coverage:** ~7% (only core navigation, actions, and onboarding title)

