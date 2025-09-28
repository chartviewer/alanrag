use crate::storage::{Storage, SearchResult};
use crate::graph::GraphBuilder;
use anyhow::Result;

pub struct HybridRetriever {
    vector_weight: f32,
    text_weight: f32,
    graph_weight: f32,
}

impl HybridRetriever {
    pub fn new(vector_weight: f32, text_weight: f32, graph_weight: f32) -> Self {
        Self {
            vector_weight,
            text_weight,
            graph_weight,
        }
    }

    pub fn retrieve(
        &self,
        storage: &Storage,
        graph: &GraphBuilder,
        query: &str,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Vec<SearchResult> {
        // Get vector search results
        let vector_results = storage.search_similar(query_embedding, top_k * 2);

        // Get text search results
        let text_results = storage.search_by_text(query, top_k * 2);

        // Combine and rerank results
        let combined = self.combine_results(vector_results, text_results, graph);

        // Take top-k
        combined.into_iter().take(top_k).collect()
    }

    fn combine_results(
        &self,
        vector_results: Vec<SearchResult>,
        text_results: Vec<SearchResult>,
        graph: &GraphBuilder,
    ) -> Vec<SearchResult> {
        use std::collections::HashMap;

        let mut combined_scores: HashMap<String, (SearchResult, f32, f32, f32)> = HashMap::new();

        // Process vector results
        for result in vector_results {
            let entry = combined_scores.entry(result.chunk_id.clone()).or_insert((
                result.clone(),
                0.0,
                0.0,
                0.0,
            ));
            entry.1 = result.score;
        }

        // Process text results
        for result in text_results {
            let entry = combined_scores.entry(result.chunk_id.clone()).or_insert((
                result.clone(),
                0.0,
                0.0,
                0.0,
            ));
            entry.2 = result.score;
        }

        // Calculate graph scores
        for (chunk_id, entry) in &mut combined_scores {
            let related_chunks = graph.find_related_chunks(chunk_id, 2);
            let graph_score = related_chunks.len() as f32 / 10.0; // Normalize
            entry.3 = graph_score;
        }

        // Combine scores and sort
        let mut final_results: Vec<SearchResult> = combined_scores
            .into_iter()
            .map(|(_, (mut result, vector_score, text_score, graph_score))| {
                let combined_score = self.vector_weight * vector_score
                    + self.text_weight * text_score
                    + self.graph_weight * graph_score;
                result.score = combined_score;
                result
            })
            .collect();

        final_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        final_results
    }
}