//! Sandbox diagnostic commands for inspecting execution environment.

use anyhow::Result;

use octo_engine::sandbox::{
    ExecutionTargetResolver, OctoRunMode, SandboxProfile, SandboxRouter, ToolCategory,
};

use super::types::SandboxCommands;
use super::AppState;

pub async fn handle_sandbox(action: SandboxCommands, _state: &AppState) -> Result<()> {
    match action {
        SandboxCommands::Status => show_status(),
        SandboxCommands::DryRun => show_dry_run(),
        SandboxCommands::ListBackends => list_backends(),
        SandboxCommands::Build { tag, no_cache, dev } => build_image(&tag, no_cache, dev).await,
        SandboxCommands::Cleanup { force, session } => cleanup_containers(force, session.as_deref()).await,
    }
}

fn show_status() -> Result<()> {
    let profile = SandboxProfile::resolve(false, None, None);
    let run_mode = OctoRunMode::detect();

    println!("Sandbox Status");
    println!("{}", "─".repeat(40));
    println!("  Profile:    {}", profile_display(&profile));
    println!("  Run Mode:   {}", run_mode_display(&run_mode));
    println!("  Policy:     {:?}", profile.policy());
    println!("  Timeout:    {}s", profile.timeout_secs());
    println!("  Env Pass:   {}", if profile.env_passthrough() { "full" } else { "restricted" });
    println!("  Approval:   {:?}", profile.approval_gate());
    println!("  Audit:      {:?}", profile.audit_level());

    Ok(())
}

fn show_dry_run() -> Result<()> {
    let profile = SandboxProfile::resolve(false, None, None);
    let run_mode = OctoRunMode::detect();
    let router = SandboxRouter::with_policy(profile.policy());
    let available = router.registered_backends();
    let resolver = ExecutionTargetResolver::new(run_mode.clone(), profile.clone(), available);

    println!("Sandbox Routing Dry-Run");
    println!("{}", "─".repeat(60));
    println!("  Profile: {}  |  Mode: {}", profile_display(&profile), run_mode_display(&run_mode));
    println!();

    let categories = [
        ("Shell",      ToolCategory::Shell),
        ("Compute",    ToolCategory::Compute),
        ("FileSystem", ToolCategory::FileSystem),
        ("Network",    ToolCategory::Network),
        ("Script",     ToolCategory::Script),
        ("Gpu",        ToolCategory::Gpu),
        ("Untrusted",  ToolCategory::Untrusted),
    ];

    println!("  {:<12} {:<25} {}", "Category", "Target", "Reason");
    println!("  {:<12} {:<25} {}", "────────", "──────", "──────");

    for (label, cat) in &categories {
        let preview = resolver.dry_run(cat.clone());
        println!("  {:<12} {:<25} {}", label, format!("{}", preview.target), preview.reason);
    }

    Ok(())
}

fn list_backends() -> Result<()> {
    let profile = SandboxProfile::resolve(false, None, None);
    let router = SandboxRouter::with_policy(profile.policy());
    let backends = router.registered_backends();

    println!("Registered Sandbox Backends");
    println!("{}", "─".repeat(40));

    if backends.is_empty() {
        println!("  (none — all execution is local)");
    } else {
        for backend in &backends {
            println!("  - {:?}", backend);
        }
    }

    println!();
    println!("Profile: {}", profile_display(&profile));
    println!("Policy:  {:?}", profile.policy());

    Ok(())
}

async fn build_image(tag: &str, no_cache: bool, dev: bool) -> Result<()> {
    let dockerfile = if dev {
        "container/Dockerfile.dev"
    } else {
        "container/Dockerfile"
    };

    // Check that the Dockerfile exists
    if !std::path::Path::new(dockerfile).exists() {
        anyhow::bail!(
            "Dockerfile not found at '{}'. Run this command from the project root.",
            dockerfile
        );
    }

    println!("Building sandbox image: {}", tag);
    println!("  Dockerfile: {}", dockerfile);
    if no_cache {
        println!("  Cache: disabled");
    }
    println!();

    let mut cmd = tokio::process::Command::new("docker");
    cmd.arg("build")
        .arg("-t")
        .arg(tag)
        .arg("-f")
        .arg(dockerfile)
        .arg("container/");

    if no_cache {
        cmd.arg("--no-cache");
    }

    // Stream output directly to the terminal
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());

    let status = cmd.status().await?;
    if status.success() {
        println!();
        println!("Image built successfully: {}", tag);
    } else {
        anyhow::bail!("Docker build failed with exit code: {:?}", status.code());
    }

    Ok(())
}

async fn cleanup_containers(force: bool, session: Option<&str>) -> Result<()> {
    println!("Cleaning up Octo sandbox containers...");

    let mut cmd = tokio::process::Command::new("docker");
    cmd.arg("ps")
        .arg("-a")
        .arg("--filter")
        .arg("label=octo-sandbox=true")
        .arg("--format")
        .arg("{{.ID}}\t{{.Names}}\t{{.Status}}");

    let output = cmd.output().await?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to list containers: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    if lines.is_empty() {
        println!("  No Octo sandbox containers found.");
        return Ok(());
    }

    let mut removed = 0;
    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let container_id = parts[0];
        let name = parts[1];

        // If session filter is set, only remove matching containers
        if let Some(sid) = session {
            if !name.contains(sid) {
                continue;
            }
        }

        // If not force, only remove stopped containers
        if !force && parts[2].starts_with("Up") {
            println!("  Skipping running container: {} ({})", name, container_id);
            continue;
        }

        println!("  Removing: {} ({})", name, container_id);
        let rm_status = tokio::process::Command::new("docker")
            .args(["rm", "-f", container_id])
            .output()
            .await?;

        if rm_status.status.success() {
            removed += 1;
        } else {
            eprintln!(
                "  Warning: failed to remove {}: {}",
                container_id,
                String::from_utf8_lossy(&rm_status.stderr)
            );
        }
    }

    println!();
    println!("Removed {} container(s).", removed);
    Ok(())
}

fn profile_display(profile: &SandboxProfile) -> &'static str {
    match profile {
        SandboxProfile::Development => "development",
        SandboxProfile::Staging => "staging",
        SandboxProfile::Production => "production",
        SandboxProfile::Custom(_) => "custom",
    }
}

fn run_mode_display(mode: &OctoRunMode) -> &'static str {
    match mode {
        OctoRunMode::Host => "host",
        OctoRunMode::Sandboxed => "sandboxed (container)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_display() {
        assert_eq!(profile_display(&SandboxProfile::Development), "development");
        assert_eq!(profile_display(&SandboxProfile::Staging), "staging");
        assert_eq!(profile_display(&SandboxProfile::Production), "production");
    }

    #[test]
    fn test_run_mode_display() {
        assert_eq!(run_mode_display(&OctoRunMode::Host), "host");
        assert_eq!(run_mode_display(&OctoRunMode::Sandboxed), "sandboxed (container)");
    }

    #[test]
    fn test_show_status_runs() {
        let result = show_status();
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_dry_run_runs() {
        let result = show_dry_run();
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_backends_runs() {
        let result = list_backends();
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_dockerfile_check() {
        // build_image should fail if Dockerfile doesn't exist at expected path
        // (we can't actually run docker build in tests)
        let path = std::path::Path::new("container/Dockerfile");
        // Just verify we can check the path
        let _ = path.exists();
    }
}
