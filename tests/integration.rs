use std::path::PathBuf;
use tempfile::TempDir;

use rag_mcp_server::config::Config;
use rag_mcp_server::mcp::McpServer;

fn create_test_config() -> Config {
    Config {
        storage: rag_mcp_server::config::StorageConfig {
            data_dir: PathBuf::from("./test_data"),
            max_chunk_size: 512,
            min_chunk_size: 100,
        },
        chunking: rag_mcp_server::config::ChunkingConfig {
            overlap_tokens: 50,
            semantic_threshold: 0.75,
            code_languages: vec!["rust".to_string(), "python".to_string()],
        },
        embedding: rag_mcp_server::config::EmbeddingConfig {
            model_name: "test-model".to_string(),
            dimension: 384,
            batch_size: 32,
        },
        mcp: rag_mcp_server::config::McpConfig {
            host: "127.0.0.1".to_string(),
            port: 3001,
        },
        graph: rag_mcp_server::config::GraphConfig {
            max_connections: 10,
            similarity_threshold: 0.7,
        },
    }
}

#[test]
fn test_mcp_server_creation() {
    let config = create_test_config();
    let server = McpServer::new(config);
    assert!(server.is_ok(), "Failed to create MCP server: {:?}", server.err());
}

#[test]
fn test_config_serialization() {
    let config = create_test_config();
    let yaml = serde_yaml::to_string(&config).unwrap();
    assert!(yaml.contains("data_dir"));
    assert!(yaml.contains("max_chunk_size"));
    assert!(yaml.contains("model_name"));
}

#[test]
fn test_chunk_operations() {
    use rag_mcp_server::chunker::{SemanticChunker, ChunkType};

    let chunker = SemanticChunker::new(512, 100, 50);
    let test_text = "This is a test document. It has multiple sentences. Each sentence provides information. The chunker should process this correctly.";

    let result = chunker.chunk_text(test_text, "test.txt");
    assert!(result.is_ok(), "Chunking failed: {:?}", result.err());

    let chunks = result.unwrap();
    assert!(!chunks.is_empty(), "No chunks created");

    for chunk in &chunks {
        assert!(!chunk.content.is_empty());
        assert!(!chunk.id.is_empty());
        assert!(matches!(chunk.metadata.chunk_type, ChunkType::Text));
    }
}