use anyhow::Result;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: ChunkMetadata,
    pub boundaries: (usize, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub source_file: String,
    pub chunk_type: ChunkType,
    pub chapter: Option<String>,
    pub section: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkType {
    Text,
    Code,
    Markdown,
    Pdf,
}

pub struct SemanticChunker {
    max_chunk_size: usize,
    min_chunk_size: usize,
    overlap_tokens: usize,
}

impl SemanticChunker {
    pub fn new(max_chunk_size: usize, min_chunk_size: usize, overlap_tokens: usize) -> Self {
        Self {
            max_chunk_size,
            min_chunk_size,
            overlap_tokens,
        }
    }

    pub fn chunk_text(&self, text: &str, source_file: &str) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();
        let sentences = self.split_sentences(text);

        let mut current_chunk = String::new();
        let mut start_pos = 0;
        let mut current_pos = 0;

        for sentence in sentences {
            if current_chunk.len() + sentence.len() > self.max_chunk_size && !current_chunk.is_empty() {
                if current_chunk.len() >= self.min_chunk_size {
                    let chunk = Chunk {
                        id: Uuid::new_v4().to_string(),
                        content: current_chunk.clone(),
                        embedding: vec![], // Will be filled by embedder
                        metadata: ChunkMetadata {
                            source_file: source_file.to_string(),
                            chunk_type: ChunkType::Text,
                            chapter: None,
                            section: None,
                            language: None,
                        },
                        boundaries: (start_pos, current_pos),
                    };
                    chunks.push(chunk);
                }

                // Start new chunk with overlap
                let overlap_start = current_chunk.len().saturating_sub(self.overlap_tokens);
                current_chunk = current_chunk[overlap_start..].to_string();
                start_pos = current_pos - (current_chunk.len() - overlap_start);
            }

            current_chunk.push_str(&sentence);
            current_pos += sentence.len();
        }

        // Add final chunk
        if !current_chunk.is_empty() && current_chunk.len() >= self.min_chunk_size {
            let chunk = Chunk {
                id: Uuid::new_v4().to_string(),
                content: current_chunk,
                embedding: vec![],
                metadata: ChunkMetadata {
                    source_file: source_file.to_string(),
                    chunk_type: ChunkType::Text,
                    chapter: None,
                    section: None,
                    language: None,
                },
                boundaries: (start_pos, current_pos),
            };
            chunks.push(chunk);
        }

        Ok(chunks)
    }

    pub fn chunk_code(&self, code: &str, language: &str, source_file: &str) -> Result<Vec<Chunk>> {
        // Simple code chunking by functions/classes for now
        // In a full implementation, this would use tree-sitter for proper AST parsing
        let lines: Vec<&str> = code.lines().collect();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut start_line = 0;

        for (i, line) in lines.iter().enumerate() {
            // Detect function/class boundaries (simplified)
            let is_function_start = line.trim_start().starts_with("fn ") ||
                                  line.trim_start().starts_with("def ") ||
                                  line.trim_start().starts_with("function ") ||
                                  line.trim_start().starts_with("class ");

            if is_function_start && !current_chunk.is_empty() {
                if current_chunk.len() >= self.min_chunk_size {
                    let chunk = Chunk {
                        id: Uuid::new_v4().to_string(),
                        content: current_chunk.clone(),
                        embedding: vec![],
                        metadata: ChunkMetadata {
                            source_file: source_file.to_string(),
                            chunk_type: ChunkType::Code,
                            chapter: None,
                            section: None,
                            language: Some(language.to_string()),
                        },
                        boundaries: (start_line, i),
                    };
                    chunks.push(chunk);
                }
                current_chunk.clear();
                start_line = i;
            }

            current_chunk.push_str(line);
            current_chunk.push('\n');

            if current_chunk.len() > self.max_chunk_size {
                if current_chunk.len() >= self.min_chunk_size {
                    let chunk = Chunk {
                        id: Uuid::new_v4().to_string(),
                        content: current_chunk.clone(),
                        embedding: vec![],
                        metadata: ChunkMetadata {
                            source_file: source_file.to_string(),
                            chunk_type: ChunkType::Code,
                            chapter: None,
                            section: None,
                            language: Some(language.to_string()),
                        },
                        boundaries: (start_line, i + 1),
                    };
                    chunks.push(chunk);
                }
                current_chunk.clear();
                start_line = i + 1;
            }
        }

        // Add final chunk
        if !current_chunk.is_empty() && current_chunk.len() >= self.min_chunk_size {
            let chunk = Chunk {
                id: Uuid::new_v4().to_string(),
                content: current_chunk,
                embedding: vec![],
                metadata: ChunkMetadata {
                    source_file: source_file.to_string(),
                    chunk_type: ChunkType::Code,
                    chapter: None,
                    section: None,
                    language: Some(language.to_string()),
                },
                boundaries: (start_line, lines.len()),
            };
            chunks.push(chunk);
        }

        Ok(chunks)
    }

    fn split_sentences(&self, text: &str) -> Vec<String> {
        // Simple sentence splitting - in production would use proper NLP
        text.split(". ")
            .map(|s| s.to_string())
            .collect()
    }
}