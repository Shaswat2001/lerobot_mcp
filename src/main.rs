mod error;
mod hub;
mod server;
mod tools;
mod cli;

use clap::{Parser};
use tracing_subscriber::{EnvFilter, fmt};

use hub::client::HubClient;
use server::LeRobotServer;
use cli::{Transport, LogFormat, Cli, Command};

fn init_tracing(format: &LogFormat) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // All logging goes to stderr. stdout is reserved for MCP JSON-RPC.
    match format {
        LogFormat::Pretty => {
            fmt()
                .with_env_filter(filter)
                .with_writer(std::io::stderr)
                .with_target(false)
                .init();
        }
        LogFormat::Json => {
            fmt()
                .with_env_filter(filter)
                .with_writer(std::io::stderr)
                .json()
                .init();
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_tracing(&cli.log_format);

    tracing::info!(
        transport = ?cli.transport,
        port = cli.port,
        cache_dir = %cli.cache_dir.display(),
        cache_ttl = cli.cache_ttl,
        hf_token_set = cli.hf_token.is_some(),
        "Starting lerobot-mcp"
    );

    let client = HubClient::new(cli.hf_token.as_deref())
        .expect("Failed to create Hub client");

    // let server: LeRobotServer = LeRobotServer::new(client);

    match cli.command {
        Some(Command::Search { query, robot_type, min_episodes, limit }) => {
            let result = tools::search::execute_search(
                &client,
                &query,
                robot_type.as_deref(),
                min_episodes,
                limit,
            )
            .await?;
            print!("{}", result.to_markdown());
        }
        Some(Command::Serve) | None => {
            let server = LeRobotServer::new(client);
            match cli.transport {
                Transport::Stdio => {
                    tracing::info!("Serving over stdio");
                    let transport = rmcp::transport::io::stdio();
                    let service = rmcp::ServiceExt::serve(server, transport).await?;
                    service.waiting().await?;
                }
                Transport::Http => {
                    tracing::error!("HTTP transport not yet implemented");
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_defaults() {
        // Simulate: lerobot-mcp (no args, all defaults)
        let cli = Cli::parse_from(["lerobot-mcp"]);
        assert!(matches!(cli.transport, Transport::Stdio));
        assert_eq!(cli.port, 8080);
        assert!(cli.hf_token.is_none());
        assert_eq!(cli.cache_ttl, 300);
    }

    #[test]
    fn cli_parses_explicit_values() {
        let cli = Cli::parse_from([
            "lerobot-mcp",
            "--transport", "http",
            "--port", "3000",
            "--hf-token", "hf_abc123",
            "--cache-ttl", "600",
            "--log-format", "json",
        ]);
        assert!(matches!(cli.transport, Transport::Http));
        assert_eq!(cli.port, 3000);
        assert_eq!(cli.hf_token.as_deref(), Some("hf_abc123"));
        assert_eq!(cli.cache_ttl, 600);
        assert!(matches!(cli.log_format, LogFormat::Json));
    }

    #[test]
    fn cli_rejects_invalid_transport() {
        let result = Cli::try_parse_from(["lerobot-mcp", "--transport", "websocket"]);
        assert!(result.is_err());
    }
}