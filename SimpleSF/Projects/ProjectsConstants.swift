import SwiftUI

// Ref: FT-SSF-003

// MARK: - Pilot Projects (AC validation — matches SF legacy)

let pilotProjects: [(name: String, tech: String, description: String)] = [
    ("Design System WCAG AA",
     "React, TypeScript, CSS",
     "30+ composants React TypeScript accessibles, design tokens CSS, documentation Storybook"),
    ("SDK Mobile Universel",
     "React Native, Expo, TypeScript",
     "Composants UI cross-platform, auth biometric, navigation, state management"),
    ("Plateforme ML Distribuée",
     "Python, PyTorch, FastAPI",
     "Training multi-worker, hyperparameter tuning, model registry, serving API REST"),
    ("Orchestrateur Workflows Data",
     "Python, Airflow, React",
     "DAG visual, scheduling CRON, retry logic, monitoring temps réel, connecteurs"),
    ("Marketplace SaaS Multi-Tenant",
     "Node.js, Next.js, PostgreSQL",
     "Multi-tenancy, billing Stripe, RBAC, API REST + GraphQL, dashboard analytics"),
    ("Migration Angular → React",
     "TypeScript, Angular, React",
     "Migration progressive 50+ composants Angular 14 vers React 18 avec feature parity"),
    ("Jeu Pac-Man macOS Natif",
     "Swift, SwiftUI, SpriteKit",
     "Pac-Man native macOS avec SpriteKit, niveaux, IA fantômes, scores persistants"),
    ("API Gateway Rust",
     "Rust, Tokio, gRPC",
     "Reverse proxy haute performance, rate limiting, auth JWT, observabilité OpenTelemetry"),
]

// MARK: - 14-phase SAFe product lifecycle (matches SF legacy Value Stream)

let safePhases: [(name: String, short: String, pattern: String)] = [
    ("Idéation",            "Idéation",       "network"),
    ("Comité Stratégique",  "Comité Strat.",   "human-in-the-loop"),
    ("Constitution",        "Constitution",    "sequential"),
    ("Architecture",        "Architecture",    "aggregator"),
    ("Design System",       "Design Sys.",     "sequential"),
    ("Sprints Dev",         "Sprints Dev",     "hierarchical"),
    ("Build & Verify",      "Build & Verify",  "sequential"),
    ("Pipeline CI/CD",      "Pipeline CI",     "sequential"),
    ("Revue UX",            "Revue UX",        "loop"),
    ("Campagne QA",         "Campagne QA",     "loop"),
    ("Exécution Tests",     "Exécution",       "parallel"),
    ("Deploy Prod",         "Deploy Prod",     "human-in-the-loop"),
    ("Routage TMA",         "Routage",         "router"),
    ("Correctif & TMA",     "Correctif",       "loop"),
]

func simulatedActivePhase(for status: ProjectStatus, progress: Double) -> Int {
    switch status {
    case .idea:     return 0
    case .planning: return 1
    case .active:   return max(2, min(13, Int(progress * 13.0)))
    case .paused:   return max(2, min(13, Int(progress * 13.0)))
    case .done:     return 14
    }
}

// MARK: - Safe Array subscript

extension Array {
    subscript(safe index: Int) -> Element? {
        indices.contains(index) ? self[index] : nil
    }
}
