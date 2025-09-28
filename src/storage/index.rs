use crate::chunker::Chunk;
use anyhow::{Result, anyhow};
use std::path::Path;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk_id: String,
    pub score: f32,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

pub struct Storage {
    chunk_store: sled::Db,
    metadata_store: sled::Db,
    embeddings: HashMap<String, Vec<f32>>, // In-memory for now
    data_dir: std::path::PathBuf,
}

impl Storage {
    pub fn new(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;

        let chunk_store = sled::open(data_dir.join("chunks"))?;
        let metadata_store = sled::open(data_dir.join("metadata"))?;

        Ok(Self {
            chunk_store,
            metadata_store,
            embeddings: HashMap::new(),
            data_dir: data_dir.to_path_buf(),
        })
    }

    pub fn store_chunk(&mut self, chunk: &Chunk) -> Result<()> {
        // Store chunk content
        let chunk_data = serde_json::to_vec(chunk)?;
        self.chunk_store.insert(&chunk.id, chunk_data)?;

        // Store metadata separately for faster lookup
        let metadata = serde_json::to_vec(&chunk.metadata)?;
        self.metadata_store.insert(&chunk.id, metadata)?;

        // Store embedding in memory
        if !chunk.embedding.is_empty() {
            self.embeddings.insert(chunk.id.clone(), chunk.embedding.clone());
        }

        self.chunk_store.flush()?;
        self.metadata_store.flush()?;

        Ok(())
    }

    pub fn get_chunk(&self, chunk_id: &str) -> Result<Option<Chunk>> {
        if let Some(data) = self.chunk_store.get(chunk_id)? {
            let chunk: Chunk = serde_json::from_slice(&data)?;
            Ok(Some(chunk))
        } else {
            Ok(None)
        }
    }

    pub fn search_similar(&self, query_embedding: &[f32], top_k: usize) -> Vec<SearchResult> {
        let mut similarities = Vec::new();

        for (chunk_id, embedding) in &self.embeddings {
            let similarity = self.cosine_similarity(query_embedding, embedding);
            similarities.push((chunk_id.clone(), similarity));
        }

        // Sort by similarity (descending)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top-k and convert to SearchResult
        similarities
            .into_iter()
            .take(top_k)
            .filter_map(|(chunk_id, score)| {
                self.get_chunk(&chunk_id).ok().flatten().map(|chunk| SearchResult {
                    chunk_id: chunk.id,
                    score,
                    content: chunk.content,
                    metadata: self.chunk_metadata_to_map(&chunk.metadata),
                })
            })
            .collect()
    }

    pub fn search_by_text(&self, query: &str, top_k: usize) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let mut total_chunks = 0;

        eprintln!("Debug: Starting text search for query: '{}'", query);
        eprintln!("Debug: Starting iteration over chunk_store");

        for chunk_result in self.chunk_store.iter() {
            eprintln!("Debug: Got chunk_result: {:?}", chunk_result.is_ok());
            if let Ok((chunk_id, chunk_data)) = chunk_result {
                if let Ok(chunk) = serde_json::from_slice::<Chunk>(&chunk_data) {
                    total_chunks += 1;
                    // Simple text matching - in production would use better text search
                    let score = self.text_similarity(&chunk.content, query);
                    eprintln!("Debug: Chunk {} score: {} (content preview: {})",
                             String::from_utf8_lossy(&chunk_id), score,
                             &chunk.content.chars().take(50).collect::<String>());
                    if score > 0.0 {
                        results.push(SearchResult {
                            chunk_id: String::from_utf8_lossy(&chunk_id).to_string(),
                            score,
                            content: chunk.content,
                            metadata: self.chunk_metadata_to_map(&chunk.metadata),
                        });
                    }
                }
            }
        }

        eprintln!("Debug: Found {} chunks total, {} with score > 0", total_chunks, results.len());

        // Sort by score (descending)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().take(top_k).collect()
    }

    pub fn get_chunks_by_file(&self, file_path: &str) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();

        for chunk_result in self.chunk_store.iter() {
            if let Ok((_, chunk_data)) = chunk_result {
                if let Ok(chunk) = serde_json::from_slice::<Chunk>(&chunk_data) {
                    if chunk.metadata.source_file == file_path {
                        chunks.push(chunk);
                    }
                }
            }
        }

        // Sort by boundaries to maintain order
        chunks.sort_by_key(|c| c.boundaries.0);
        Ok(chunks)
    }

    pub fn get_chunks_by_chapter(&self, file_path: &str, chapter: &str) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();

        for chunk_result in self.chunk_store.iter() {
            if let Ok((_, chunk_data)) = chunk_result {
                if let Ok(chunk) = serde_json::from_slice::<Chunk>(&chunk_data) {
                    if chunk.metadata.source_file == file_path {
                        if let Some(chunk_chapter) = &chunk.metadata.chapter {
                            if chunk_chapter == chapter {
                                chunks.push(chunk);
                            }
                        }
                    }
                }
            }
        }

        chunks.sort_by_key(|c| c.boundaries.0);
        Ok(chunks)
    }

    pub fn list_files(&self) -> Result<Vec<String>> {
        let mut files = std::collections::HashSet::new();

        for chunk_result in self.chunk_store.iter() {
            if let Ok((_, chunk_data)) = chunk_result {
                if let Ok(chunk) = serde_json::from_slice::<Chunk>(&chunk_data) {
                    files.insert(chunk.metadata.source_file);
                }
            }
        }

        Ok(files.into_iter().collect())
    }

    pub fn list_chapters(&self, file_path: &str) -> Result<Vec<String>> {
        let mut chapters = std::collections::HashSet::new();

        for chunk_result in self.chunk_store.iter() {
            if let Ok((_, chunk_data)) = chunk_result {
                if let Ok(chunk) = serde_json::from_slice::<Chunk>(&chunk_data) {
                    if chunk.metadata.source_file == file_path {
                        if let Some(chapter) = &chunk.metadata.chapter {
                            chapters.insert(chapter.clone());
                        }
                    }
                }
            }
        }

        Ok(chapters.into_iter().collect())
    }

    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }

    fn text_similarity(&self, text: &str, query: &str) -> f32 {
        let text_lower = text.to_lowercase();
        let query_lower = query.to_lowercase();

        // Simple keyword matching score
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let matched_words = query_words
            .iter()
            .filter(|word| text_lower.contains(*word))
            .count();

        if query_words.is_empty() {
            0.0
        } else {
            matched_words as f32 / query_words.len() as f32
        }
    }

    fn chunk_metadata_to_map(&self, metadata: &crate::chunker::ChunkMetadata) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("source_file".to_string(), metadata.source_file.clone());
        map.insert("chunk_type".to_string(), format!("{:?}", metadata.chunk_type));

        if let Some(chapter) = &metadata.chapter {
            map.insert("chapter".to_string(), chapter.clone());
        }

        if let Some(section) = &metadata.section {
            map.insert("section".to_string(), section.clone());
        }

        if let Some(language) = &metadata.language {
            map.insert("language".to_string(), language.clone());
        }

        map
    }
}