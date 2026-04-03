//! Bash command classifier for per-command risk assessment.
//!
//! Parses the first command in a pipeline/chain and classifies it by risk level
//! and category, enabling the PermissionEngine to make granular decisions.

use grid_types::RiskLevel;

/// Classification result for a bash command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandClassification {
    pub risk: CommandRisk,
    pub category: CommandCategory,
    pub reason: String,
}

/// Risk level for a classified command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRisk {
    /// Safe read-only commands (ls, cat, echo, grep, find without -exec)
    ReadOnly,
    /// Low-risk writes (touch, mkdir, cp, file creation)
    LowRisk,
    /// Medium-risk operations (git commit, npm install, cargo build)
    MediumRisk,
    /// High-risk operations (rm -rf, chmod, chown, git push, docker)
    HighRisk,
    /// Dangerous operations (rm -rf /, dd, mkfs, systemctl, iptables)
    Dangerous,
}

/// Category of the command for grouping and permission control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    FileRead,
    FileWrite,
    FileDelete,
    ProcessManagement,
    NetworkAccess,
    PackageManagement,
    VersionControl,
    SystemAdmin,
    Container,
    Unknown,
}

impl CommandRisk {
    /// Map command risk to the `grid_types::RiskLevel` enum.
    ///
    /// `RiskLevel` has four variants: ReadOnly, LowRisk, HighRisk, Destructive.
    /// We collapse MediumRisk into LowRisk since there is no MediumRisk variant.
    pub fn to_risk_level(self) -> RiskLevel {
        match self {
            Self::ReadOnly => RiskLevel::ReadOnly,
            Self::LowRisk | Self::MediumRisk => RiskLevel::LowRisk,
            Self::HighRisk => RiskLevel::HighRisk,
            Self::Dangerous => RiskLevel::Destructive,
        }
    }
}

/// Classify a bash command string by analyzing the first command in a pipeline.
pub fn classify_command(command: &str) -> CommandClassification {
    let trimmed = command.trim();

    if trimmed.is_empty() {
        return CommandClassification {
            risk: CommandRisk::ReadOnly,
            category: CommandCategory::Unknown,
            reason: "empty command".into(),
        };
    }

    let first_cmd = extract_first_command(trimmed);
    let base_cmd = extract_base_command(&first_cmd);
    let args_lower = first_cmd.to_lowercase();

    // Check dangerous patterns first (highest priority)
    if let Some(c) = check_dangerous_patterns(&args_lower) {
        return c;
    }

    classify_by_command(base_cmd, &args_lower)
}

/// Extract the first command from a pipeline (cmd1 | cmd2) or chain (cmd1 && cmd2).
fn extract_first_command(cmd: &str) -> String {
    let first = cmd
        .split(&['|', '&', ';'][..])
        .next()
        .unwrap_or(cmd)
        .trim();
    first.to_string()
}

/// Extract the base command name, skipping env assignments and wrappers like sudo.
fn extract_base_command(cmd: &str) -> &str {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    for part in &parts {
        if part.contains('=') {
            continue; // skip VAR=value
        }
        match *part {
            "sudo" | "nice" | "nohup" | "time" | "env" | "xargs" => continue,
            _ => return part,
        }
    }
    parts.first().copied().unwrap_or("")
}

fn check_dangerous_patterns(args: &str) -> Option<CommandClassification> {
    let dangerous_patterns: &[(&str, &str)] = &[
        ("rm -rf /", "recursive force delete on root"),
        ("rm -rf /*", "recursive force delete on root contents"),
        ("rm -rf ~", "recursive force delete on home"),
        ("dd if=/dev/", "raw disk operation"),
        ("mkfs", "filesystem format"),
        ("> /dev/sd", "raw disk write"),
        (":(){ :|:& };:", "fork bomb"),
        ("chmod -r 777 /", "global permission change"),
    ];

    for (pattern, reason) in dangerous_patterns {
        if args.contains(pattern) {
            return Some(CommandClassification {
                risk: CommandRisk::Dangerous,
                category: CommandCategory::SystemAdmin,
                reason: reason.to_string(),
            });
        }
    }
    None
}

fn classify_by_command(base: &str, args: &str) -> CommandClassification {
    match base {
        // === ReadOnly ===
        "ls" | "cat" | "head" | "tail" | "less" | "more" | "wc" | "echo" | "printf" | "pwd"
        | "date" | "whoami" | "id" | "hostname" | "uname" | "df" | "du" | "file" | "stat"
        | "which" | "type" | "man" | "help" | "true" | "false" | "test" | "[" => {
            CommandClassification {
                risk: CommandRisk::ReadOnly,
                category: CommandCategory::FileRead,
                reason: format!("{base}: read-only command"),
            }
        }

        "grep" | "rg" | "ag" | "ack" | "find" | "fd" | "locate" | "tree" => {
            if args.contains("-exec") || args.contains("-delete") {
                CommandClassification {
                    risk: CommandRisk::MediumRisk,
                    category: CommandCategory::FileWrite,
                    reason: format!("{base}: search with side effects"),
                }
            } else {
                CommandClassification {
                    risk: CommandRisk::ReadOnly,
                    category: CommandCategory::FileRead,
                    reason: format!("{base}: search command"),
                }
            }
        }

        // === LowRisk (file creation/modification) ===
        "touch" | "mkdir" | "cp" | "tee" | "patch" | "sed" | "awk" => CommandClassification {
            risk: CommandRisk::LowRisk,
            category: CommandCategory::FileWrite,
            reason: format!("{base}: file creation/modification"),
        },

        // === MediumRisk (build/test/install) ===
        "cargo" | "npm" | "npx" | "yarn" | "pnpm" | "pip" | "uv" | "python" | "python3"
        | "node" | "make" | "cmake" | "rustc" | "gcc" | "g++" | "go" | "java" | "javac" => {
            CommandClassification {
                risk: CommandRisk::MediumRisk,
                category: CommandCategory::PackageManagement,
                reason: format!("{base}: build/package tool"),
            }
        }

        "git" => classify_git(args),

        // === HighRisk (deletion, permissions) ===
        "rm" => {
            if args.contains("-rf") || args.contains("-r") {
                CommandClassification {
                    risk: CommandRisk::HighRisk,
                    category: CommandCategory::FileDelete,
                    reason: "rm: recursive deletion".into(),
                }
            } else {
                CommandClassification {
                    risk: CommandRisk::MediumRisk,
                    category: CommandCategory::FileDelete,
                    reason: "rm: file deletion".into(),
                }
            }
        }

        "mv" => CommandClassification {
            risk: CommandRisk::MediumRisk,
            category: CommandCategory::FileWrite,
            reason: "mv: file move/rename".into(),
        },

        "chmod" | "chown" | "chgrp" => CommandClassification {
            risk: CommandRisk::HighRisk,
            category: CommandCategory::SystemAdmin,
            reason: format!("{base}: permission/ownership change"),
        },

        // === Network ===
        "curl" | "wget" | "fetch" => CommandClassification {
            risk: CommandRisk::MediumRisk,
            category: CommandCategory::NetworkAccess,
            reason: format!("{base}: network request"),
        },

        "ssh" | "scp" | "rsync" | "nc" | "ncat" | "nmap" | "telnet" => CommandClassification {
            risk: CommandRisk::HighRisk,
            category: CommandCategory::NetworkAccess,
            reason: format!("{base}: remote access"),
        },

        // === Container ===
        "docker" | "podman" | "kubectl" | "helm" => CommandClassification {
            risk: CommandRisk::HighRisk,
            category: CommandCategory::Container,
            reason: format!("{base}: container operation"),
        },

        // === SystemAdmin (dangerous) ===
        "systemctl" | "service" | "init" | "reboot" | "shutdown" | "halt" | "iptables"
        | "firewall-cmd" | "mount" | "umount" | "fdisk" | "kill" | "killall" | "pkill" => {
            CommandClassification {
                risk: CommandRisk::Dangerous,
                category: CommandCategory::SystemAdmin,
                reason: format!("{base}: system administration"),
            }
        }

        // === Process monitoring (read-only) ===
        "ps" | "top" | "htop" | "free" | "uptime" | "vmstat" | "iostat" | "netstat" | "ss"
        | "lsof" | "strace" | "ltrace" => CommandClassification {
            risk: CommandRisk::ReadOnly,
            category: CommandCategory::ProcessManagement,
            reason: format!("{base}: process/system monitoring"),
        },

        // === Default: unknown command -> MediumRisk ===
        _ => CommandClassification {
            risk: CommandRisk::MediumRisk,
            category: CommandCategory::Unknown,
            reason: format!("{base}: unclassified command"),
        },
    }
}

fn classify_git(args: &str) -> CommandClassification {
    if args.contains("push") || args.contains("force") {
        CommandClassification {
            risk: CommandRisk::HighRisk,
            category: CommandCategory::VersionControl,
            reason: "git push: remote modification".into(),
        }
    } else if args.contains("reset --hard") || args.contains("clean -f") {
        CommandClassification {
            risk: CommandRisk::HighRisk,
            category: CommandCategory::VersionControl,
            reason: "git: destructive operation".into(),
        }
    } else if args.contains("commit")
        || args.contains("merge")
        || args.contains("rebase")
        || args.contains("checkout")
    {
        CommandClassification {
            risk: CommandRisk::MediumRisk,
            category: CommandCategory::VersionControl,
            reason: "git: state-changing operation".into(),
        }
    } else {
        CommandClassification {
            risk: CommandRisk::ReadOnly,
            category: CommandCategory::VersionControl,
            reason: "git: read-only operation".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- ReadOnly --

    #[test]
    fn test_readonly_ls() {
        let c = classify_command("ls -la");
        assert_eq!(c.risk, CommandRisk::ReadOnly);
        assert_eq!(c.category, CommandCategory::FileRead);
    }

    #[test]
    fn test_readonly_cat() {
        let c = classify_command("cat /etc/hosts");
        assert_eq!(c.risk, CommandRisk::ReadOnly);
        assert_eq!(c.category, CommandCategory::FileRead);
    }

    #[test]
    fn test_readonly_echo() {
        let c = classify_command("echo hello world");
        assert_eq!(c.risk, CommandRisk::ReadOnly);
        assert_eq!(c.category, CommandCategory::FileRead);
    }

    #[test]
    fn test_readonly_grep() {
        let c = classify_command("grep -r pattern src/");
        assert_eq!(c.risk, CommandRisk::ReadOnly);
        assert_eq!(c.category, CommandCategory::FileRead);
    }

    #[test]
    fn test_readonly_git_log() {
        let c = classify_command("git log --oneline -10");
        assert_eq!(c.risk, CommandRisk::ReadOnly);
        assert_eq!(c.category, CommandCategory::VersionControl);
    }

    #[test]
    fn test_readonly_git_status() {
        let c = classify_command("git status");
        assert_eq!(c.risk, CommandRisk::ReadOnly);
        assert_eq!(c.category, CommandCategory::VersionControl);
    }

    #[test]
    fn test_readonly_git_diff() {
        let c = classify_command("git diff HEAD~1");
        assert_eq!(c.risk, CommandRisk::ReadOnly);
        assert_eq!(c.category, CommandCategory::VersionControl);
    }

    #[test]
    fn test_readonly_ps() {
        let c = classify_command("ps aux");
        assert_eq!(c.risk, CommandRisk::ReadOnly);
        assert_eq!(c.category, CommandCategory::ProcessManagement);
    }

    #[test]
    fn test_readonly_find() {
        let c = classify_command("find . -name '*.rs'");
        assert_eq!(c.risk, CommandRisk::ReadOnly);
        assert_eq!(c.category, CommandCategory::FileRead);
    }

    // -- LowRisk --

    #[test]
    fn test_lowrisk_touch() {
        let c = classify_command("touch newfile.txt");
        assert_eq!(c.risk, CommandRisk::LowRisk);
        assert_eq!(c.category, CommandCategory::FileWrite);
    }

    #[test]
    fn test_lowrisk_mkdir() {
        let c = classify_command("mkdir -p src/new_module");
        assert_eq!(c.risk, CommandRisk::LowRisk);
        assert_eq!(c.category, CommandCategory::FileWrite);
    }

    #[test]
    fn test_lowrisk_cp() {
        let c = classify_command("cp file1.txt file2.txt");
        assert_eq!(c.risk, CommandRisk::LowRisk);
        assert_eq!(c.category, CommandCategory::FileWrite);
    }

    // -- MediumRisk --

    #[test]
    fn test_medium_cargo_build() {
        let c = classify_command("cargo build --release");
        assert_eq!(c.risk, CommandRisk::MediumRisk);
        assert_eq!(c.category, CommandCategory::PackageManagement);
    }

    #[test]
    fn test_medium_npm_install() {
        let c = classify_command("npm install express");
        assert_eq!(c.risk, CommandRisk::MediumRisk);
        assert_eq!(c.category, CommandCategory::PackageManagement);
    }

    #[test]
    fn test_medium_git_commit() {
        let c = classify_command("git commit -m 'test'");
        assert_eq!(c.risk, CommandRisk::MediumRisk);
        assert_eq!(c.category, CommandCategory::VersionControl);
    }

    #[test]
    fn test_medium_curl() {
        let c = classify_command("curl https://example.com");
        assert_eq!(c.risk, CommandRisk::MediumRisk);
        assert_eq!(c.category, CommandCategory::NetworkAccess);
    }

    #[test]
    fn test_medium_rm_single() {
        let c = classify_command("rm file.txt");
        assert_eq!(c.risk, CommandRisk::MediumRisk);
        assert_eq!(c.category, CommandCategory::FileDelete);
    }

    #[test]
    fn test_medium_mv() {
        let c = classify_command("mv old.txt new.txt");
        assert_eq!(c.risk, CommandRisk::MediumRisk);
        assert_eq!(c.category, CommandCategory::FileWrite);
    }

    #[test]
    fn test_medium_find_exec() {
        let c = classify_command("find . -name '*.tmp' -exec rm {} \\;");
        assert_eq!(c.risk, CommandRisk::MediumRisk);
        assert_eq!(c.category, CommandCategory::FileWrite);
    }

    #[test]
    fn test_medium_unknown_command() {
        let c = classify_command("some_custom_tool --flag");
        assert_eq!(c.risk, CommandRisk::MediumRisk);
        assert_eq!(c.category, CommandCategory::Unknown);
    }

    // -- HighRisk --

    #[test]
    fn test_high_rm_rf() {
        let c = classify_command("rm -rf build/");
        assert_eq!(c.risk, CommandRisk::HighRisk);
        assert_eq!(c.category, CommandCategory::FileDelete);
    }

    #[test]
    fn test_high_rm_recursive() {
        let c = classify_command("rm -r old_dir");
        assert_eq!(c.risk, CommandRisk::HighRisk);
        assert_eq!(c.category, CommandCategory::FileDelete);
    }

    #[test]
    fn test_high_chmod() {
        let c = classify_command("chmod 755 script.sh");
        assert_eq!(c.risk, CommandRisk::HighRisk);
        assert_eq!(c.category, CommandCategory::SystemAdmin);
    }

    #[test]
    fn test_high_git_push() {
        let c = classify_command("git push origin main");
        assert_eq!(c.risk, CommandRisk::HighRisk);
        assert_eq!(c.category, CommandCategory::VersionControl);
    }

    #[test]
    fn test_high_git_reset_hard() {
        let c = classify_command("git reset --hard HEAD~1");
        assert_eq!(c.risk, CommandRisk::HighRisk);
        assert_eq!(c.category, CommandCategory::VersionControl);
    }

    #[test]
    fn test_high_docker() {
        let c = classify_command("docker run -it ubuntu bash");
        assert_eq!(c.risk, CommandRisk::HighRisk);
        assert_eq!(c.category, CommandCategory::Container);
    }

    #[test]
    fn test_high_ssh() {
        let c = classify_command("ssh user@host");
        assert_eq!(c.risk, CommandRisk::HighRisk);
        assert_eq!(c.category, CommandCategory::NetworkAccess);
    }

    // -- Dangerous --

    #[test]
    fn test_dangerous_rm_rf_root() {
        let c = classify_command("rm -rf /");
        assert_eq!(c.risk, CommandRisk::Dangerous);
        assert_eq!(c.category, CommandCategory::SystemAdmin);
    }

    #[test]
    fn test_dangerous_rm_rf_home() {
        let c = classify_command("rm -rf ~");
        assert_eq!(c.risk, CommandRisk::Dangerous);
        assert_eq!(c.category, CommandCategory::SystemAdmin);
    }

    #[test]
    fn test_dangerous_systemctl() {
        let c = classify_command("systemctl restart nginx");
        assert_eq!(c.risk, CommandRisk::Dangerous);
        assert_eq!(c.category, CommandCategory::SystemAdmin);
    }

    #[test]
    fn test_dangerous_kill() {
        let c = classify_command("kill -9 1234");
        assert_eq!(c.risk, CommandRisk::Dangerous);
        assert_eq!(c.category, CommandCategory::SystemAdmin);
    }

    #[test]
    fn test_dangerous_dd() {
        let c = classify_command("dd if=/dev/zero of=/dev/sda");
        assert_eq!(c.risk, CommandRisk::Dangerous);
        assert_eq!(c.category, CommandCategory::SystemAdmin);
    }

    #[test]
    fn test_dangerous_reboot() {
        let c = classify_command("reboot");
        assert_eq!(c.risk, CommandRisk::Dangerous);
        assert_eq!(c.category, CommandCategory::SystemAdmin);
    }

    #[test]
    fn test_dangerous_iptables() {
        let c = classify_command("iptables -F");
        assert_eq!(c.risk, CommandRisk::Dangerous);
        assert_eq!(c.category, CommandCategory::SystemAdmin);
    }

    // -- Pipeline parsing --

    #[test]
    fn test_pipeline_classifies_first_command() {
        let c = classify_command("ls -la | grep foo");
        assert_eq!(c.risk, CommandRisk::ReadOnly);
        assert_eq!(c.category, CommandCategory::FileRead);
    }

    #[test]
    fn test_chain_classifies_first_command() {
        let c = classify_command("echo hello && rm -rf /tmp/junk");
        assert_eq!(c.risk, CommandRisk::ReadOnly);
        assert_eq!(c.category, CommandCategory::FileRead);
    }

    #[test]
    fn test_semicolon_classifies_first_command() {
        let c = classify_command("pwd; ls");
        assert_eq!(c.risk, CommandRisk::ReadOnly);
        assert_eq!(c.category, CommandCategory::FileRead);
    }

    // -- Sudo stripping --

    #[test]
    fn test_sudo_rm_rf_root() {
        let c = classify_command("sudo rm -rf /");
        assert_eq!(c.risk, CommandRisk::Dangerous);
        assert_eq!(c.category, CommandCategory::SystemAdmin);
    }

    #[test]
    fn test_sudo_ls() {
        let c = classify_command("sudo ls /root");
        assert_eq!(c.risk, CommandRisk::ReadOnly);
        assert_eq!(c.category, CommandCategory::FileRead);
    }

    // -- Empty command --

    #[test]
    fn test_empty_command() {
        let c = classify_command("");
        assert_eq!(c.risk, CommandRisk::ReadOnly);
        assert_eq!(c.category, CommandCategory::Unknown);
    }

    #[test]
    fn test_whitespace_only() {
        let c = classify_command("   ");
        assert_eq!(c.risk, CommandRisk::ReadOnly);
        assert_eq!(c.category, CommandCategory::Unknown);
    }

    // -- risk_level conversion --

    #[test]
    fn test_risk_to_risk_level() {
        assert_eq!(CommandRisk::ReadOnly.to_risk_level(), RiskLevel::ReadOnly);
        assert_eq!(CommandRisk::LowRisk.to_risk_level(), RiskLevel::LowRisk);
        assert_eq!(CommandRisk::MediumRisk.to_risk_level(), RiskLevel::LowRisk);
        assert_eq!(CommandRisk::HighRisk.to_risk_level(), RiskLevel::HighRisk);
        assert_eq!(
            CommandRisk::Dangerous.to_risk_level(),
            RiskLevel::Destructive
        );
    }

    // -- Env var prefix --

    #[test]
    fn test_env_var_prefix_skipped() {
        let c = classify_command("RUST_LOG=debug cargo test");
        assert_eq!(c.risk, CommandRisk::MediumRisk);
        assert_eq!(c.category, CommandCategory::PackageManagement);
    }

    // -- Git subcommand classification --

    #[test]
    fn test_git_clean_f() {
        let c = classify_command("git clean -fd");
        assert_eq!(c.risk, CommandRisk::HighRisk);
        assert_eq!(c.category, CommandCategory::VersionControl);
    }

    #[test]
    fn test_git_checkout() {
        let c = classify_command("git checkout feature-branch");
        assert_eq!(c.risk, CommandRisk::MediumRisk);
        assert_eq!(c.category, CommandCategory::VersionControl);
    }

    #[test]
    fn test_git_rebase() {
        let c = classify_command("git rebase main");
        assert_eq!(c.risk, CommandRisk::MediumRisk);
        assert_eq!(c.category, CommandCategory::VersionControl);
    }

    // -- Nohup / time wrappers --

    #[test]
    fn test_nohup_wrapper() {
        let c = classify_command("nohup python3 server.py");
        assert_eq!(c.risk, CommandRisk::MediumRisk);
        assert_eq!(c.category, CommandCategory::PackageManagement);
    }

    #[test]
    fn test_time_wrapper() {
        let c = classify_command("time cargo build");
        assert_eq!(c.risk, CommandRisk::MediumRisk);
        assert_eq!(c.category, CommandCategory::PackageManagement);
    }
}
