import Foundation

// Ref: FT-SSF-015
// All localization string keys organized by feature.
// Usage: L10n.t(.nav.jarvis) or L10n.t(.common.save)

enum StringKey: String, CaseIterable, Sendable {

    // MARK: - App
    case appName = "app.name"
    case appTagline = "app.tagline"

    // MARK: - Navigation
    case navJarvis = "nav.jarvis"
    case navProjects = "nav.projects"
    case navIdeation = "nav.ideation"
    case navSettings = "nav.settings"
    case navSoftwareFactory = "nav.software_factory"

    // MARK: - Common actions
    case actionStart = "action.start"
    case actionStop = "action.stop"
    case actionPause = "action.pause"
    case actionCancel = "action.cancel"
    case actionSave = "action.save"
    case actionDelete = "action.delete"
    case actionRetry = "action.retry"
    case actionBack = "action.back"
    case actionContinue = "action.continue"
    case actionRefresh = "action.refresh"
    case actionUse = "action.use"
    case actionTest = "action.test"
    case actionSkip = "action.skip"
    case actionGetStarted = "action.get_started"
    case actionLaunch = "action.launch"

    // MARK: - Status
    case statusRunning = "status.running"
    case statusStopped = "status.stopped"
    case statusActive = "status.active"
    case statusReady = "status.ready"
    case statusQueued = "status.queued"
    case statusPaused = "status.paused"
    case statusCompleted = "status.completed"
    case statusLoading = "status.loading"
    case statusStarting = "status.starting"
    case statusNotConfigured = "status.not_configured"

    // MARK: - Sidebar
    case sidebarYolo = "sidebar.yolo"
    case sidebarYoloHelp = "sidebar.yolo_help"
    case sidebarNoLLM = "sidebar.no_llm"
    case sidebarAutoDetect = "sidebar.auto_detect"
    case sidebarAutoDetectHelp = "sidebar.auto_detect_help"

    // MARK: - Jarvis
    case jarvisTitle = "jarvis.title"
    case jarvisThinking = "jarvis.thinking"
    case jarvisTeamMeeting = "jarvis.team_meeting"
    case jarvisWelcome = "jarvis.welcome"
    case jarvisNewConversation = "jarvis.new_conversation"

    // MARK: - Projects
    case projectsTitle = "projects.title"
    case projectsEmpty = "projects.empty"
    case projectsEmptyHint = "projects.empty_hint"
    case projectsPilot = "projects.pilot"
    case projectsACValidation = "projects.ac_validation"
    case projectsLoadPilot = "projects.load_pilot"
    case projectsReset = "projects.reset"
    case projectsLoadPilotHint = "projects.load_pilot_hint"
    case projectsLifecycle = "projects.lifecycle"
    case projectsWorkflowNotDefined = "projects.workflow_not_defined"
    case projectsLaunchWorkflow = "projects.launch_workflow"
    case projectsPhaseResult = "projects.phase_result"
    case projectsAgentWorking = "projects.agent_working"
    case projectsAgentWriting = "projects.agent_writing"
    case projectsMessages = "projects.messages"
    case projectsRound = "projects.round"

    // MARK: - Mission
    case missionTitle = "mission.title"
    case missionValueStream = "mission.value_stream"
    case missionLaunchEpic = "mission.launch_epic"
    case missionDescription = "mission.description"
    case missionSelectProject = "mission.select_project"
    case missionLaunchSAFe = "mission.launch_safe"
    case missionPhaseResult = "mission.phase_result"

    // MARK: - Ideation
    case ideationTitle = "ideation.title"
    case ideationDiscussing = "ideation.discussing"
    case ideationDescribe = "ideation.describe"
    case ideationAgents = "ideation.agents"
    case ideationRounds = "ideation.rounds"
    case ideationPattern = "ideation.pattern"
    case ideationLaunch = "ideation.launch"
    case ideationConfigureProvider = "ideation.configure_provider"
    case ideationComplete = "ideation.complete"

    // MARK: - Agents
    case agentsTitle = "agents.title"
    case agentsCount = "agents.count"

    // MARK: - Settings / Onboarding
    case settingsTitle = "settings.title"
    case settingsLanguage = "settings.language"
    case settingsLanguageHint = "settings.language_hint"
    case settingsLocalModels = "settings.local_models"
    case settingsLocalHint = "settings.local_hint"
    case settingsCloudProviders = "settings.cloud_providers"
    case settingsCloudHint = "settings.cloud_hint"
    case settingsNoProvider = "settings.no_provider"
    case settingsNoModelActive = "settings.no_model_active"
    case settingsSelectProvider = "settings.select_provider"
    case settingsConfigureHint = "settings.configure_hint"
    case settingsAutoStart = "settings.auto_start"
    case settingsGetAPIKey = "settings.get_api_key"
    case settingsModel = "settings.model"
    case settingsPort = "settings.port"

    // MARK: - Onboarding / Setup
    case setupTitle = "setup.title"
    case setupSubtitle = "setup.subtitle"
    case setupHardware = "setup.hardware"
    case setupHowRun = "setup.how_run"
    case setupPrivacy = "setup.privacy"
    case setupChooseModel = "setup.choose_model"
    case setupMemory = "setup.memory"
    case setupRecommended = "setup.recommended"
    case setupInstalled = "setup.installed"
    case setupModelDetails = "setup.model_details"
    case setupDownloadComplete = "setup.download_complete"
    case setupDownloadFailed = "setup.download_failed"
    case setupDownloading = "setup.downloading"
    case setupDownloadSize = "setup.download_size"
    case setupOllama = "setup.ollama"
    case setupOllamaAvailable = "setup.ollama_available"
    case setupOllamaNoModels = "setup.ollama_no_models"
    case setupOllamaNotRunning = "setup.ollama_not_running"
    case setupOllamaRunning = "setup.ollama_running"
    case setupOllamaContinue = "setup.ollama_continue"
    case setupOllamaOpen = "setup.ollama_open"
    case setupOllamaInstall = "setup.ollama_install"
    case setupAllSet = "setup.all_set"
    case setupAllSetSubtitle = "setup.all_set_subtitle"
    case setupStartUsing = "setup.start_using"
    case setupSkipConfigure = "setup.skip_configure"
    case setupSkipCloud = "setup.skip_cloud"
    case setupUseModel = "setup.use_model"
    case setupDownloadModel = "setup.download_model"
    case setupOllamaMLX = "setup.ollama_mlx"
    case setupAppleMLX = "setup.apple_mlx"
    case setupNoMLXModels = "setup.no_mlx_models"
    case setupNoOllamaModels = "setup.no_ollama_models"
    case setupStartOllama = "setup.start_ollama"
    case setupStartServer = "setup.start_server"

    // MARK: - Privacy
    case privacyLocal = "privacy.local"
    case privacyCloud = "privacy.cloud"

    // MARK: - Output
    case outputExportZip = "output.export_zip"
    case outputPushGit = "output.push_git"

    // MARK: - Accessibility
    case a11yLoading = "a11y.loading"
    case a11yLoadingAgents = "a11y.loading_agents"
    case a11yLoadingProjects = "a11y.loading_projects"
    case a11yLoadingChat = "a11y.loading_chat"
    case a11yLoadingMission = "a11y.loading_mission"
    case a11yError = "a11y.error"
    case a11yOffline = "a11y.offline"

    // MARK: - Plurals (use with L10n.plural())
    case pluralAgents = "plural.agents"
    case pluralProjects = "plural.projects"
    case pluralMessages = "plural.messages"
    case pluralPhases = "plural.phases"
    case pluralRounds = "plural.rounds"

    // MARK: - Language picker
    case languagePickerTitle = "language_picker.title"
    case languagePickerSystem = "language_picker.system"
    case languagePickerSearch = "language_picker.search"
}
