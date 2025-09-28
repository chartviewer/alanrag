use jsonrpc_core::IoHandler;
use std::sync::Arc;

use super::server::{McpServer, RagMcp};

pub fn create_rpc_handler(server: Arc<McpServer>) -> IoHandler {
    let mut io = IoHandler::new();
    io.extend_with((*server).clone().to_delegate());
    io
}

pub async fn start_mcp_server(server: Arc<McpServer>) -> anyhow::Result<()> {
    use jsonrpc_stdio_server::ServerBuilder;

    let io = create_rpc_handler(server);

    tracing::info!("MCP server starting on stdio");

    // Start stdio server
    let server = ServerBuilder::new(io)
        .build();

    // Handle requests from stdin and write responses to stdout
    server.await;

    Ok(())
}