use jsonrpc_core::{Value, Error as JsonRpcError};
use jsonrpc_derive::rpc;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::json;
use anyhow::Result;

use crate::storage::{Storage, SearchResult};
use crate::chunker::{SemanticChunker, pdf::PdfProcessor, markdown::MarkdownProcessor, text::TextProcessor, code::CodeProcessor};
use crate::graph::GraphBuilder;
use crate::storage::embeddings::EmbeddingModel;
use crate::config::Config;

#[rpc]
pub trait RagMcp {
    #[rpc(name = "ingest")]
    fn ingest(&self, path: String, doc_type: Option<String>) -> Result<Value, JsonRpcError>;

    #[rpc(name = "search_knowledge_chunk")]
    fn search_knowledge_chunk(&self, query: String, top_k: Option<usize>) -> Result<Value, JsonRpcError>;

    #[rpc(name = "search_knowledge_chapter")]
    fn search_knowledge_chapter(&self, query: String, top_k: Option<usize>) -> Result<Value, JsonRpcError>;
}

#[derive(Clone)]
pub struct McpServer {
    storage: Arc<RwLock<Storage>>,
    chunker: Arc<SemanticChunker>,
    graph: Arc<RwLock<GraphBuilder>>,
    embedder: Arc<EmbeddingModel>,
    config: Config,
}

impl McpServer {
    pub fn new(config: Config) -> Result<Self> {
        let storage = Arc::new(RwLock::new(Storage::new(&config.storage.data_dir)?));

        let chunker = Arc::new(SemanticChunker::new(
            config.storage.max_chunk_size,
            config.storage.min_chunk_size,
            config.chunking.overlap_tokens,
        ));

        let graph = Arc::new(RwLock::new(GraphBuilder::new(
            config.graph.similarity_threshold,
        )));

        let embedder = Arc::new(EmbeddingModel::new(
            &config.embedding.model_name,
            config.embedding.dimension,
        )?);

        Ok(Self {
            storage,
            chunker,
            graph,
            embedder,
            config,
        })
    }

    async fn process_document(&self, path: &str, doc_type: Option<&str>) -> Result<usize> {
        // Determine document type
        let detected_type = doc_type.unwrap_or_else(|| {
            match std::path::Path::new(path).extension().and_then(|s| s.to_str()) {
                Some("pdf") => "pdf",
                Some("md") | Some("markdown") => "markdown",
                Some("txt") => "text",
                Some(ext) if CodeProcessor::detect_language(path).is_some() => "code",
                _ => "text"
            }
        });

        // Read file content
        let content = if detected_type == "pdf" {
            // PDF processing handled separately
            String::new()
        } else {
            std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?
        };

        // Process based on type
        let mut chunks = match detected_type {
            "pdf" => PdfProcessor::extract_and_chunk(path, &self.chunker)?,
            "markdown" => MarkdownProcessor::extract_and_chunk(&content, path, &self.chunker)?,
            "code" => {
                let language = CodeProcessor::detect_language(path).unwrap_or_else(|| "text".to_string());
                CodeProcessor::extract_and_chunk(&content, &language, path, &self.chunker)?
            },
            _ => TextProcessor::extract_and_chunk(&content, path, &self.chunker)?,
        };

        // Generate embeddings for chunks
        for chunk in &mut chunks {
            let embedding = self.embedder.embed_text(&chunk.content)?;
            chunk.embedding = embedding;
        }

        // Store chunks
        let chunk_count = chunks.len();
        {
            let mut storage = self.storage.write().await;
            for chunk in &chunks {
                storage.store_chunk(chunk)?;
            }
        }

        // Build graph relationships
        {
            let mut graph = self.graph.write().await;
            graph.build_relationships(&chunks)?;
        }

        Ok(chunk_count)
    }

    async fn search_chunks(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        // Generate query embedding
        let query_embedding = self.embedder.embed_text(query)?;

        // Search for similar chunks
        let storage = self.storage.read().await;
        let mut results = storage.search_similar(&query_embedding, top_k * 2); // Get more for reranking

        // If vector search doesn't find enough results, fallback to text search
        if results.len() < top_k {
            let mut text_results = storage.search_by_text(query, top_k);
            results.append(&mut text_results);

            // Remove duplicates and sort by score
            results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            results.dedup_by(|a, b| a.chunk_id == b.chunk_id);
        }

        // Apply graph-based reranking if needed
        results = self.apply_graph_reranking(results).await;

        Ok(results.into_iter().take(top_k).collect())
    }

    async fn search_chapters(&self, query: &str, top_k: usize) -> Result<Vec<Value>> {
        // First find relevant chunks - get more results to ensure we capture chapters
        let chunk_results = self.search_chunks(query, top_k * 5).await?;

        // Group by chapter and aggregate scores
        let mut chapter_scores: std::collections::HashMap<String, (f32, Vec<SearchResult>)> = std::collections::HashMap::new();

        let chunk_results_clone = chunk_results.clone();

        for result in chunk_results {
            // Check both chapter and section fields for chapter information
            let chapter_name = result.metadata.get("chapter")
                .or_else(|| result.metadata.get("section"))
                .cloned();

            if let Some(chapter) = chapter_name {
                let source_file = result.metadata.get("source_file")
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");
                let chapter_key = format!("{}#{}", source_file, chapter);
                let entry = chapter_scores.entry(chapter_key).or_insert((0.0, Vec::new()));
                entry.0 += result.score;
                entry.1.push(result);
            }
        }

        // Also try to include chunks that don't have explicit chapter metadata but are relevant
        // This helps with documents that might not have perfect chapter extraction
        if chapter_scores.is_empty() {
            // If no chapters found, group by section or file as fallback
            for result in &chunk_results_clone[0..std::cmp::min(chunk_results_clone.len(), top_k * 2)] {
                let section_or_file = result.metadata.get("section")
                    .or_else(|| result.metadata.get("source_file"))
                    .cloned()
                    .unwrap_or_else(|| "Unknown Section".to_string());

                let source_file = result.metadata.get("source_file")
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");
                let chapter_key = format!("{}#{}", source_file, section_or_file);
                let entry = chapter_scores.entry(chapter_key).or_insert((0.0, Vec::new()));
                entry.0 += result.score;
                entry.1.push(result.clone());
            }
        }

        // Sort chapters by aggregated score
        let mut sorted_chapters: Vec<_> = chapter_scores.into_iter().collect();
        sorted_chapters.sort_by(|a, b| b.1.0.partial_cmp(&a.1.0).unwrap_or(std::cmp::Ordering::Equal));

        // Return chapter information
        let results: Vec<Value> = sorted_chapters
            .into_iter()
            .take(top_k)
            .map(|(chapter_key, (score, chunks))| {
                let parts: Vec<&str> = chapter_key.split('#').collect();
                let file_path = parts.get(0).unwrap_or(&"unknown");
                let chapter_name = parts.get(1).unwrap_or(&"unknown");

                // Average the score by number of chunks for better ranking
                let avg_score = if chunks.is_empty() { 0.0 } else { score / chunks.len() as f32 };

                json!({
                    "chapter": chapter_name,
                    "file": file_path,
                    "score": avg_score,
                    "total_score": score,
                    "chunk_count": chunks.len(),
                    "chunks": chunks.iter().map(|c| json!({
                        "id": c.chunk_id,
                        "content": c.content,
                        "score": c.score,
                        "metadata": c.metadata
                    })).collect::<Vec<_>>()
                })
            })
            .collect();

        Ok(results)
    }

    async fn apply_graph_reranking(&self, results: Vec<SearchResult>) -> Vec<SearchResult> {
        // For now, return results as-is
        // In a full implementation, this would use graph relationships to boost related content
        results
    }
}

impl RagMcp for McpServer {
    fn ingest(&self, path: String, doc_type: Option<String>) -> Result<Value, JsonRpcError> {
        // Use a blocking approach to avoid runtime conflicts
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.process_document(&path, doc_type.as_deref()).await
            })
        });

        match result {
            Ok(chunk_count) => Ok(json!({
                "status": "success",
                "chunks_created": chunk_count,
                "document_path": path
            })),
            Err(e) => {
                let mut error = JsonRpcError::internal_error();
                error.message = format!("Ingestion failed: {}", e);
                error.data = Some(json!({"path": path}));
                Err(error)
            }
        }
    }

    fn search_knowledge_chunk(&self, query: String, top_k: Option<usize>) -> Result<Value, JsonRpcError> {
        let k = top_k.unwrap_or(10);

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.search_chunks(&query, k).await
            })
        });

        match result {
            Ok(results) => Ok(json!({
                "query": query,
                "chunks": results.iter().map(|r| json!({
                    "id": r.chunk_id,
                    "content": r.content,
                    "score": r.score,
                    "metadata": r.metadata
                })).collect::<Vec<_>>(),
                "total_found": results.len()
            })),
            Err(e) => {
                let mut error = JsonRpcError::internal_error();
                error.message = format!("Search failed: {}", e);
                error.data = Some(json!({"query": query}));
                Err(error)
            }
        }
    }

    fn search_knowledge_chapter(&self, query: String, top_k: Option<usize>) -> Result<Value, JsonRpcError> {
        let k = top_k.unwrap_or(5);

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.search_chapters(&query, k).await
            })
        });

        match result {
            Ok(chapters) => Ok(json!({
                "query": query,
                "chapters": chapters,
                "total_found": chapters.len()
            })),
            Err(e) => {
                let mut error = JsonRpcError::internal_error();
                error.message = format!("Chapter search failed: {}", e);
                error.data = Some(json!({"query": query}));
                Err(error)
            }
        }
    }
}