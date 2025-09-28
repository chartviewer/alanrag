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
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = Config::from_file("rag_config.yaml")
        .or_else(|_| Config::from_file("../rag_config.yaml"))
        .unwrap_or_else(|e| {
            error!("Failed to load configuration: {}", e);
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
