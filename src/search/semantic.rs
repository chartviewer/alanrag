use crate::storage::{Storage, SearchResult};
use anyhow::Result;

pub struct SemanticSearch {
    threshold: f32,
}

impl SemanticSearch {
    pub fn new(threshold: f32) -> Self {
        Self { threshold }
    }

    pub fn search_with_expansion(&self, storage: &Storage, query_embedding: &[f32], top_k: usize) -> Vec<SearchResult> {
        // Get initial results
        let mut results = storage.search_similar(query_embedding, top_k * 2);

        // Filter by threshold
        results.retain(|r| r.score >= self.threshold);

        // Sort and take top-k
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().take(top_k).collect()
    }

    pub fn rerank_with_diversity(&self, results: Vec<SearchResult>, diversity_factor: f32) -> Vec<SearchResult> {
        if results.len() <= 1 {
            return results;
        }

        let results_len = results.len();
        let mut reranked = Vec::new();
        let mut remaining = results;

        // Take the best result first
        if let Some(best) = remaining.first() {
            reranked.push(best.clone());
            remaining.remove(0);
        }

        // For remaining results, balance relevance and diversity
        while !remaining.is_empty() && reranked.len() < results_len {
            let mut best_idx = 0;
            let mut best_score = 0.0;

            for (i, candidate) in remaining.iter().enumerate() {
                // Calculate diversity penalty
                let mut diversity_penalty = 0.0;
                for selected in &reranked {
                    let similarity = self.text_similarity(&candidate.content, &selected.content);
                    diversity_penalty += similarity;
                }

                let avg_diversity_penalty = if reranked.is_empty() {
                    0.0
                } else {
                    diversity_penalty / reranked.len() as f32
                };

                // Combine relevance and diversity
                let final_score = candidate.score * (1.0 - diversity_factor * avg_diversity_penalty);

                if final_score > best_score {
                    best_score = final_score;
                    best_idx = i;
                }
            }

            reranked.push(remaining.remove(best_idx));
        }

        reranked
    }

    fn text_similarity(&self, text1: &str, text2: &str) -> f32 {
        let words1: std::collections::HashSet<&str> = text1.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = text2.split_whitespace().collect();

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }
}