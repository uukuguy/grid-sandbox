//! Command type definitions for Octo CLI

use clap::Subcommand;

/// Agent subcommands
#[derive(Subcommand)]
pub enum AgentCommands {
    /// List all available agents
    List,

    /// Run an agent for interactive conversation
    Run {
        /// Agent ID to run
        #[arg(value_name = "AGENT_ID")]
        agent_id: Option<String>,
    },

    /// Show agent details
    Info {
        /// Agent ID
        #[arg(value_name = "AGENT_ID")]
        agent_id: String,
    },
}

/// Session subcommands
#[derive(Subcommand)]
pub enum SessionCommands {
    /// List all sessions
    List,

    /// Create a new session
    Create {
        /// Session name (optional)
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Show session details
    Show {
        /// Session ID
        #[arg(value_name = "SESSION_ID")]
        session_id: String,
    },

    /// Delete a session
    Delete {
        /// Session ID
        #[arg(value_name = "SESSION_ID")]
        session_id: String,
    },
}

/// Memory subcommands
#[derive(Subcommand)]
pub enum MemoryCommands {
    /// Search memory
    Search {
        /// Search query
        #[arg(value_name = "QUERY")]
        query: String,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// List recent memories
    List {
        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Add a memory entry
    Add {
        /// Memory content
        #[arg(value_name = "CONTENT")]
        content: String,

        /// Memory tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
    },
}

/// Tools subcommands
#[derive(Subcommand)]
pub enum ToolsCommands {
    /// List all available tools
    List,

    /// Invoke a tool
    Invoke {
        /// Tool name
        #[arg(value_name = "TOOL_NAME")]
        tool_name: String,

        /// Tool arguments as JSON
        #[arg(value_name = "ARGS")]
        args: Option<String>,
    },

    /// Show tool details
    Info {
        /// Tool name
        #[arg(value_name = "TOOL_NAME")]
        tool_name: String,
    },
}

/// Config subcommands
#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Validate configuration
    Validate,
}
