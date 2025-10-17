use crate::chunker::Chunk;
use anyhow::{Result, anyhow};
use std::path::Path;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
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
    embeddings: Arc<RwLock<HashMap<String, Vec<f32>>>>, // Thread-safe in-memory cache
    data_dir: std::path::PathBuf,
}

impl Storage {
    pub fn new(data_dir: &Path) -> Result<Self> {
        Self::new_with_instance(data_dir, None)
    }

    pub fn new_with_instance(data_dir: &Path, instance_id: Option<&str>) -> Result<Self> {
        // If instance_id is provided, create a subdirectory for this instance
        // This allows multiple MCP servers to run with isolated databases
        let effective_data_dir = if let Some(id) = instance_id {
            data_dir.join(format!("instance_{}", id))
        } else {
            data_dir.to_path_buf()
        };

        std::fs::create_dir_all(&effective_data_dir)?;

        // Sled doesn't support multi-process access to the same database directory.
        // Each MCP server instance must have its own database directory.
        let chunk_config = sled::Config::new()
            .path(effective_data_dir.join("chunks"))
            .flush_every_ms(Some(100))  // Auto-flush every 100ms for crash consistency
            .cache_capacity(128 * 1024 * 1024);  // 128MB cache for better performance

        let metadata_config = sled::Config::new()
            .path(effective_data_dir.join("metadata"))
            .flush_every_ms(Some(100))
            .cache_capacity(32 * 1024 * 1024);  // 32MB cache

        let chunk_store = chunk_config.open()
            .map_err(|e| anyhow!("Failed to open chunk store at {:?}: {}. Is another instance already running with the same data_dir?", effective_data_dir.join("chunks"), e))?;
        let metadata_store = metadata_config.open()
            .map_err(|e| anyhow!("Failed to open metadata store at {:?}: {}. Is another instance already running with the same data_dir?", effective_data_dir.join("metadata"), e))?;

        // Load existing embeddings from disk into memory cache
        let mut embeddings = HashMap::new();
        for chunk_result in chunk_store.iter() {
            if let Ok((chunk_id, chunk_data)) = chunk_result {
                if let Ok(chunk) = serde_json::from_slice::<Chunk>(&chunk_data) {
                    if !chunk.embedding.is_empty() {
                        embeddings.insert(
                            String::from_utf8_lossy(&chunk_id).to_string(),
                            chunk.embedding.clone()
                        );
                    }
                }
            }
        }

        Ok(Self {
            chunk_store,
            metadata_store,
            embeddings: Arc::new(RwLock::new(embeddings)),
            data_dir: effective_data_dir,
        })
    }

    pub fn store_chunk(&self, chunk: &Chunk) -> Result<()> {
        // Store chunk content
        let chunk_data = serde_json::to_vec(chunk)?;
        self.chunk_store.insert(&chunk.id, chunk_data)?;

        // Store metadata separately for faster lookup
        let metadata = serde_json::to_vec(&chunk.metadata)?;
        self.metadata_store.insert(&chunk.id, metadata)?;

        // Store embedding in memory cache (thread-safe)
        if !chunk.embedding.is_empty() {
            let mut embeddings = self.embeddings.write().unwrap();
            embeddings.insert(chunk.id.clone(), chunk.embedding.clone());
        }

        // Sled handles its own flushing, no need to call flush explicitly
        // This improves performance and reduces lock contention

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

        // Read lock on embeddings cache for concurrent access
        let embeddings = self.embeddings.read().unwrap();
        for (chunk_id, embedding) in embeddings.iter() {
            let similarity = self.cosine_similarity(query_embedding, embedding);
            similarities.push((chunk_id.clone(), similarity));
        }
        drop(embeddings); // Release read lock early

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
        // Enhanced text similarity with BM25-like scoring
        let text_lower = text.to_lowercase();
        let query_lower = query.to_lowercase();

        // Tokenize both text and query
        let text_words: Vec<&str> = text_lower.split_whitespace().collect();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        if query_words.is_empty() {
            return 0.0;
        }

        // Calculate term frequencies and match scores
        let mut total_score = 0.0;
        let k1 = 1.2; // BM25 parameter
        let b = 0.75; // BM25 parameter
        let avg_doc_length = 500.0; // Average document length in words
        let doc_length = text_words.len() as f32;

        for query_word in &query_words {
            // Count occurrences of query word in text
            let term_freq = text_words.iter().filter(|w| *w == query_word).count() as f32;

            if term_freq > 0.0 {
                // BM25 term frequency component
                let tf_component = (term_freq * (k1 + 1.0)) /
                    (term_freq + k1 * (1.0 - b + b * doc_length / avg_doc_length));

                // IDF component (simplified - in production would use corpus statistics)
                let idf = 1.0; // Simplified IDF

                total_score += tf_component * idf;
            }
        }

        // Normalize score
        total_score / query_words.len() as f32
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