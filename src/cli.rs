use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, ValueEnum)]
pub enum Transport {
    /// Communicate over stdin/stdout (default, used by Claude Desktop / Claude Code)
    Stdio,
    /// Communicate over HTTP (for remote clients)
    Http,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum LogFormat {
    Pretty,
    Json
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Start the MCP server
    Serve,
    /// Search datasets directly (no MCP server)
    Search {
        /// Search query
        query: String,
        /// Filter by robot type
        #[arg(long)]
        robot_type: Option<String>,
        /// Minimum episodes
        #[arg(long)]
        min_episodes: Option<u32>,
        /// Max results
        #[arg(long, default_value = "10")]
        limit: u32,
    },
}

/// lerobot-mcp: A Rust MCP server for the LeRobot dataset ecosystem.
///
/// Gives any MCP-compatible LLM client conversational access to the
/// thousands of LeRobot robotics datasets on the Hugging Face Hub.
#[derive(Debug, Parser)]
#[command(name = "lerobot-mcp", version, about)]
pub struct Cli {

    #[command(subcommand)]
    pub command: Option<Command>,
    /// Transport mode
    #[arg(short, long, value_enum, default_value_t=Transport::Stdio)]
    pub transport: Transport,
 
    /// Port for HTTP transport (ignored for stdio)
    #[arg(short, long, default_value = "8080")]
    pub port: u16,
 
    /// Hugging Face API token (for private/gated datasets)
    #[arg(long, env = "HF_TOKEN")]
    pub hf_token: Option<String>,
 
    /// Cache directory
    #[arg(long, env = "LEROBOT_MCP_CACHE_DIR", default_value = "~/.cache/lerobot-mcp")]
    pub cache_dir: PathBuf,
 
    /// Default cache TTL in seconds
    #[arg(long, default_value = "300")]
    pub cache_ttl: u64,
 
    /// Log output format
    #[arg(short, long, value_enum, default_value_t = LogFormat::Pretty)]
    pub log_format: LogFormat,
}
