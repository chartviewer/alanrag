use super::{Chunk, ChunkMetadata, ChunkType};
use anyhow::Result;
use pdf_extract::extract_text;
use uuid::Uuid;

pub struct PdfProcessor;

impl PdfProcessor {
    pub fn extract_and_chunk(file_path: &str, chunker: &super::SemanticChunker) -> Result<Vec<Chunk>> {
        let text = extract_text(file_path)?;

        let mut chunks = chunker.chunk_text(&text, file_path)?;

        // Update chunk metadata to indicate PDF source
        for chunk in &mut chunks {
            chunk.metadata.chunk_type = ChunkType::Pdf;
        }

        Ok(chunks)
    }
}