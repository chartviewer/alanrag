use super::{Chunk, ChunkMetadata, ChunkType};
use anyhow::Result;

pub struct TextProcessor;

impl TextProcessor {
    pub fn extract_and_chunk(content: &str, file_path: &str, chunker: &super::SemanticChunker) -> Result<Vec<Chunk>> {
        chunker.chunk_text(content, file_path)
    }
}