# SimpleSF User-Facing Strings - Quick Reference

## 📋 Overview
- **Total Hardcoded Strings:** 150+
- **Already Localized:** 13 keys (7%)
- **Need Localization:** 137+
- **Supported Languages:** 30+ (in existing Localizable.xcstrings)
- **Primary Language:** Mixed English/French

## 📁 Output Files

| File | Size | Purpose |
|------|------|---------|
| `USER_FACING_STRINGS_REPORT.md` | 18 KB | Detailed report with all strings organized by file & category |
| `STRINGS_INVENTORY.json` | 6.5 KB | JSON inventory for programmatic access |
| `STRINGS_EXTRACTION_SUMMARY.txt` | 14 KB | Executive summary with findings & recommendations |
| `QUICK_REFERENCE.md` | This | Quick lookup guide |

## 🔑 Key Findings

### ⚠️ Critical Issue: French Content
Many UI strings are in **FRENCH**, not English:
- "Lancer un Epic" (Launch Epic)
- "Décrivez votre produit" (Describe your product)
- "Cycle de Vie Produit Complet" (Full Product Lifecycle)
- "Workflow non défini" (Workflow undefined)
- ~35 strings total in French

**Action:** Confirm if French should be primary UI language

### Largest Files (Most Strings)
1. `Projects/ProjectsView.swift` (73 KB) - 30 strings
2. `Onboarding/OnboardingView.swift` (26 KB) - 40 strings
3. `Onboarding/SetupWizardView.swift` (23 KB) - 50 strings
4. `Jarvis/JarvisView.swift` (27 KB) - 12 strings
5. `Mission/MissionView.swift` (681 lines) - 25 strings

## 📊 String Categories

| Category | Count | Examples |
|----------|-------|----------|
| Navigation | 7 | Jarvis, Projects, Ideation, Settings |
| Buttons | 35 | Get Started, Continue, Cancel, Use, Test, Delete |
| Labels | 20 | Language, Local Models, Cloud Providers |
| Status Messages | 25 | En cours, Terminé, Échoué, Running, Stopped |
| Empty States | 8 | No projects yet, No models installed |
| Help Text | 15 | "Everything stays on your Mac..." |
| Placeholders | 5 | Search projects, Select a project |
| Error Messages | 5 | Workspace not found, git failed |
| French Content | 35 | ⚠️ Needs review |

## 🎯 String Distribution by File

### No Hardcoded Strings (Clean)
- ✅ `App/SimpleSFApp.swift`
- ✅ `App/AppState.swift`
- ✅ `Views/Shared/ContentView.swift`
- ✅ `Views/Shared/DesignTokens.swift` (design system only)
- ✅ `Views/Shared/MarkdownView.swift` (markdown renderer only)

### Most Strings to Localize
- 📝 `Onboarding/SetupWizardView.swift` (50 strings)
- 📝 `Onboarding/OnboardingView.swift` (40 strings)
- 📝 `Projects/ProjectsView.swift` (30 strings)
- 📝 `Mission/MissionView.swift` (25 strings)
- 📝 `Jarvis/JarvisView.swift` (12 strings)
- 📝 `Ideation/IdeationView.swift` (10 strings)

## 💾 Existing i18n Structure

**File:** `SimpleSF/i18n/Localizable.xcstrings`
**Format:** JSON Apple Localizable Strings Catalog
**Languages:** 30+ (af, ar, bn, ca, cs, da, de, el, en, es, fi, fr, he, hi, hr, hu, id, it, ja, ko, lt, lv, ms, nb, nl, pl, pt, ro, ru, sk, sl, sr, sv, th, tl, tr, uk, ur, vi, zh)

### Currently Localized Keys (13)
```
app.name                 - "Simple SF"
nav.projects             - "Projects"
nav.jarvis               - "Jarvis"
nav.ideation             - "Ideation"
mode.simple              - "Simple"
mode.advanced            - "Advanced"
action.start             - "Start"
action.pause             - "Pause"
action.stop              - "Stop"
jarvis.welcome           - "Hi, I'm Jarvis"
onboarding.title         - "Configure your LLM providers"
output.export_zip        - "Export as ZIP"
output.push_git          - "Push to Git"
```

## 🚀 Top Priority Strings to Localize

### Mission/Value Stream (20+ strings)
- "Value Stream"
- "Lancer un Epic"
- "Cycle de Vie Produit Complet"
- Status: "En cours", "Terminé", "Échoué", "Véto"

### Projects (30+ strings)
- "Projects", "No projects yet"
- "Projets Pilotes"
- "Charger", "Réinitialiser"

### Onboarding (90+ strings)
- All setup wizard steps
- Model selection, downloading, completion

### Settings (20+ strings)
- "Language", "Local Models", "Ollama", "Apple MLX"

## 📝 Implementation Checklist

- [ ] Review French strings - determine primary UI language
- [ ] Create i18n keys for 137+ hardcoded strings
- [ ] Use naming convention: `section.subsection.key`
- [ ] Replace all `Text("string")` with `String(localized: "key")`
- [ ] Test localization with 2-3 languages
- [ ] Export localizations for translation
- [ ] Import translated strings
- [ ] Test UI layout with various language lengths

## 🔗 Key Patterns

### Main Navigation (from MainView.swift)
```swift
enum SidebarItem {
    case jarvis, projects, ideation, settings
}
```

### Model Status Badge (from OnboardingView.swift)
```
Active → Running → Stopped → Starting...
```

### Mission Status (from MissionView.swift)
```
En cours → Terminé
       → Échoué
       → Véto
```

### Onboarding Steps (from SetupWizardView.swift)
```
Welcome → Choose Engine → Model Selection → Downloading → 
Ollama Setup → Completion
```

## 📚 Documentation Links

- Full details: See `USER_FACING_STRINGS_REPORT.md`
- Data format: See `STRINGS_INVENTORY.json`
- Executive summary: See `STRINGS_EXTRACTION_SUMMARY.txt`

## 🏁 Quick Stats

| Metric | Value |
|--------|-------|
| Files Reviewed | 15 Swift + 1 i18n |
| Files with Strings | 12 |
| Hardcoded Strings | 150+ |
| Localized Strings | 13 (7%) |
| Strings to Localize | 137+ |
| Languages Supported | 30+ |
| Estimated Effort | 2-3 days for extraction + 1-2 weeks for translation |

---

**Last Updated:** March 14, 2025
**Extraction Tool:** Comprehensive code analysis (grep, view, bash)
**Status:** ✅ Complete - All files reviewed
