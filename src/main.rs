mod config;
mod chunker;
mod graph;
mod storage;
mod mcp;
mod search;

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, error};

use config::Config;
use mcp::{McpServer, handlers::start_mcp_server};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to stderr (stdout is used for MCP communication)
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)  // Disable ANSI colors for cleaner output
        .init();

    // Set working directory to the directory containing the executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            if let Err(e) = std::env::set_current_dir(exe_dir) {
                error!("Failed to set working directory to executable directory: {}", e);
            } else {
                info!("Set working directory to: {:?}", exe_dir);
            }
        }
    }

    // Load configuration
    let config = Config::from_file("rag_config.yaml")
        .or_else(|e| {
            let abs_path = std::fs::canonicalize("rag_config.yaml").unwrap_or_else(|_| {
                std::env::current_dir().unwrap_or_else(|_| ".".into()).join("rag_config.yaml")
            });
            error!("Failed to load rag_config.yaml: {} (absolute path: {})", e, abs_path.display());
            Config::from_file("../rag_config.yaml")
        })
        .unwrap_or_else(|e| {
            error!("Failed to load ../rag_config.yaml: {} (absolute path: {})", e, std::fs::canonicalize("../rag_config.yaml").unwrap_or_else(|_| "../rag_config.yaml".into()).display());
            error!("Current working directory: {:?}", std::env::current_dir().unwrap_or_else(|_| "unknown".into()));
            std::process::exit(1);
        });

    info!("Loaded configuration from rag_config.yaml");

    // Create data directory if it doesn't exist
    std::fs::create_dir_all(&config.storage.data_dir)?;

    // Create MCP server
    let server = McpServer::new(config.clone())?;
    let server_arc = Arc::new(server);

    info!("RAG MCP Server initialized successfully");
    info!("Storage directory: {:?}", config.storage.data_dir);
    info!("Max chunk size: {}", config.storage.max_chunk_size);
    info!("Embedding model: {}", config.embedding.model_name);

    // Start MCP server
    start_mcp_server(server_arc).await?;

    Ok(())
}
