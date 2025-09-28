use jsonrpc_core::{IoHandler, Params};
use std::sync::Arc;
use serde_json::json;

use super::server::{McpServer, RagMcp};

pub fn create_rpc_handler(server: Arc<McpServer>) -> IoHandler {
    let mut io = IoHandler::new();

    // Clone the server for use in tools/call handler
    let server_for_tools = server.clone();

    // Add other methods from the RagMcp trait first
    io.extend_with((*server).clone().to_delegate());

    // Override/Add manual handler for initialize that accepts any params
    // This will replace any existing handler with the same name
    io.add_sync_method("initialize", move |_params: Params| {
        // Return standard MCP initialize response
        Ok(json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {
                "tools": {},
                "resources": {},
                "prompts": {}
            },
            "serverInfo": {
                "name": "rag-mcp-server",
                "version": "0.1.0"
            }
        }))
    });

    // Add tools/list handler
    io.add_sync_method("tools/list", move |_params: Params| {
        // Return the list of available tools
        Ok(json!({
            "tools": [
                {
                    "name": "ingest",
                    "description": "Ingest a document into the RAG system for knowledge storage",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the document to ingest"
                            },
                            "doc_type": {
                                "type": "string",
                                "description": "Type of document (pdf, markdown, text, code)",
                                "enum": ["pdf", "markdown", "text", "code"]
                            }
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": "search_knowledge_chunk",
                    "description": "Search for relevant knowledge chunks based on a query",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The search query"
                            },
                            "top_k": {
                                "type": "integer",
                                "description": "Number of results to return",
                                "default": 10
                            }
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "search_knowledge_chapter",
                    "description": "Search for relevant chapters/sections based on a query",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The search query"
                            },
                            "top_k": {
                                "type": "integer",
                                "description": "Number of chapters to return",
                                "default": 5
                            }
                        },
                        "required": ["query"]
                    }
                }
            ]
        }))
    });

    // Add prompts/list handler
    io.add_sync_method("prompts/list", move |_params: Params| {
        // Return empty prompts list for now
        Ok(json!({
            "prompts": []
        }))
    });

    // Add resources/list handler
    io.add_sync_method("resources/list", move |_params: Params| {
        // Return empty resources list for now
        Ok(json!({
            "resources": []
        }))
    });

    // Add notifications/initialized handler
    io.add_notification("notifications/initialized", |_params: Params| {
        // This is a notification, no response needed
        // Just log that initialization is complete
        tracing::debug!("Client initialization complete");
    });

    // Add tools/call handler to invoke the actual tool methods
    io.add_method("tools/call", move |params: Params| {
        let server = server_for_tools.clone();

        async move {
            // Parse the parameters
            let params_obj = match params {
                Params::Map(map) => map,
                _ => {
                    return Err(jsonrpc_core::Error::invalid_params("Expected object parameters"));
                }
            };

            // Extract tool name and arguments
            let name = params_obj.get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing 'name' field"))?;

            let arguments = params_obj.get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));

            // Call the appropriate tool based on name
            match name {
                "ingest" => {
                    // Extract parameters for ingest
                    let path = arguments.get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing 'path' field"))?
                        .to_string();

                    let doc_type = arguments.get("doc_type")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    // Call the ingest method
                    server.ingest(path, doc_type)
                        .map(|result| json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": format!("Successfully ingested document: {}",
                                        result.get("document_path").and_then(|v| v.as_str()).unwrap_or("unknown"))
                                }
                            ]
                        }))
                }
                "search_knowledge_chunk" => {
                    // Extract parameters for search
                    let query = arguments.get("query")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing 'query' field"))?
                        .to_string();

                    let top_k = arguments.get("top_k")
                        .and_then(|v| v.as_u64())
                        .map(|k| k as usize);

                    // Call the search method
                    server.search_knowledge_chunk(query, top_k)
                        .map(|result| json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&result).unwrap_or_else(|_| "Error formatting results".to_string())
                                }
                            ]
                        }))
                }
                "search_knowledge_chapter" => {
                    // Extract parameters for chapter search
                    let query = arguments.get("query")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing 'query' field"))?
                        .to_string();

                    let top_k = arguments.get("top_k")
                        .and_then(|v| v.as_u64())
                        .map(|k| k as usize);

                    // Call the search chapter method
                    server.search_knowledge_chapter(query, top_k)
                        .map(|result| json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&result).unwrap_or_else(|_| "Error formatting results".to_string())
                                }
                            ]
                        }))
                }
                _ => {
                    Err(jsonrpc_core::Error::invalid_params(format!("Unknown tool: {}", name)))
                }
            }
        }
    });

    io
}

pub async fn start_mcp_server(server: Arc<McpServer>) -> anyhow::Result<()> {
    use tokio::io::{stdin, stdout, AsyncBufReadExt, AsyncWriteExt, BufReader};
    use std::io::Write;

    let io = create_rpc_handler(server);

    tracing::info!("MCP server starting on stdio");

    // Custom stdio handling to avoid extra newlines
    let stdin = stdin();
    let mut stdout = stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            break; // EOF
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        tracing::debug!("Processing request: {}", trimmed);

        // Process the JSON-RPC request
        match io.handle_request(trimmed).await {
            Some(response) => {
                // Write response without extra newlines
                stdout.write_all(response.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
                tracing::debug!("Sent response: {}", response);
            }
            None => {
                // No response needed (notification)
                tracing::debug!("No response needed for request");
            }
        }
    }

    Ok(())
}