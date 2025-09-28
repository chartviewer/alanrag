use super::{Chunk, ChunkMetadata, ChunkType};
use anyhow::Result;

pub struct CodeProcessor;

impl CodeProcessor {
    pub fn extract_and_chunk(content: &str, language: &str, file_path: &str, chunker: &super::SemanticChunker) -> Result<Vec<Chunk>> {
        chunker.chunk_code(content, language, file_path)
    }

    pub fn detect_language(file_path: &str) -> Option<String> {
        let extension = std::path::Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str());

        match extension {
            Some("rs") => Some("rust".to_string()),
            Some("py") => Some("python".to_string()),
            Some("js") => Some("javascript".to_string()),
            Some("ts") => Some("typescript".to_string()),
            Some("java") => Some("java".to_string()),
            Some("cpp" | "cc" | "cxx") => Some("cpp".to_string()),
            Some("c") => Some("c".to_string()),
            Some("go") => Some("go".to_string()),
            _ => None,
        }
    }
}