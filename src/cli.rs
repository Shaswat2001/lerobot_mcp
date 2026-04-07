use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum Transport {
    Stdio,
    Http
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum LogFormat {
    Pretty,
    Json
}

#[derive(Parser, Debug)]
#[command(name = "lerobot-mcp", version, about = "MCP server for LeRobot datasets")]
pub struct Config {
    #[arg(short, long, default_value = "8080")]
    pub port: u16,

    #[arg(short, long, default_value="/tmp/lerobot-mcp-cache")]
    pub cache_dir: PathBuf,

    #[arg(long, env = "HF_TOKEN")]
    pub hf_token: Option<String>,

    #[arg(short, long, value_enum, default_value_t=Transport::Stdio)]
    pub transport: Transport,

    #[arg(short, long, value_enum, default_value_t=LogFormat::Pretty)]
    pub log_format: LogFormat
}