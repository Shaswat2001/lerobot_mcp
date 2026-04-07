use clap::Parser;

mod cli;
mod error;
mod hub;

use cli::{Config, Transport};
use error::{AppError, Result};
use hub::client::HubClient;

fn validate_config(config: &Config) -> Result<()> {
    if config.transport == Transport::Stdio && config.port != 8080 {
        return Err(AppError::InvalidParam {
            message: "--port is ignored when transport is stdio".to_string(),
        });
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let config = Config::parse();
    println!("{:#?}", config);

    match validate_config(&config) {
        Ok(()) => println!("Config is valid."),
        Err(e) => println!("Warning: {}", e),
    }

    // Create the Hub client
    let client = HubClient::new(config.hf_token.as_deref());

    // Search for LeRobot datasets
    println!("\nSearching for 'aloha' datasets...\n");
    match client.search_datasets("aloha", 5).await {
        Ok(datasets) => {
            println!("Found {} datasets:\n", datasets.len());
            for ds in &datasets {
                println!("  {} ({} downloads)", ds.id, ds.downloads);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}