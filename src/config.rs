use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub storage: StorageConfig,
    pub chunking: ChunkingConfig,
    pub embedding: EmbeddingConfig,
    pub mcp: McpConfig,
    pub graph: GraphConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StorageConfig {
    pub data_dir: PathBuf,
    pub max_chunk_size: usize,
    pub min_chunk_size: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChunkingConfig {
    pub overlap_tokens: usize,
    pub semantic_threshold: f32,
    pub code_languages: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EmbeddingConfig {
    pub model_name: String,
    pub dimension: usize,
    pub batch_size: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct McpConfig {
    pub transport: String, // "stdio" or "tcp"
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GraphConfig {
    pub max_connections: usize,
    pub similarity_threshold: f32,
}

impl Config {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}