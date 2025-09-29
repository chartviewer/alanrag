use crate::storage::SearchResult;
use anyhow::Result;
use std::collections::HashMap;
use std::collections::HashSet;

/// BM25 keyword search implementation for exact term matching
pub struct BM25Search {
    // Document frequency for each term
    term_doc_freq: HashMap<String, usize>,
    // Total number of documents
    total_docs: usize,
    // Average document length
    avg_doc_length: f32,
    // BM25 parameters
    k1: f32,
    b: f32,
}

impl BM25Search {
    pub fn new() -> Self {
        Self {
            term_doc_freq: HashMap::new(),
            total_docs: 0,
            avg_doc_length: 0.0,
            k1: 1.2,  // Controls term frequency normalization
            b: 0.75,  // Controls document length normalization
        }
    }

    /// Index a document for BM25 search
    pub fn index_document(&mut self, doc_id: &str, content: &str) {
        let terms = self.tokenize(content);
        let unique_terms: HashSet<String> = terms.into_iter().collect();

        // Update document frequency for each unique term
        for term in unique_terms {
            *self.term_doc_freq.entry(term).or_insert(0) += 1;
        }

        self.total_docs += 1;

        // Update average document length
        let doc_length = self.tokenize(content).len() as f32;
        self.avg_doc_length = (self.avg_doc_length * (self.total_docs - 1) as f32 + doc_length) / self.total_docs as f32;
    }

    /// Search documents using BM25 scoring
    pub fn search(&self, query: &str, documents: &[(String, String)], top_k: usize) -> Vec<SearchResult> {
        let query_terms = self.tokenize(query);
        let mut scored_docs = Vec::new();

        for (doc_id, content) in documents {
            let score = self.calculate_bm25_score(&query_terms, content);
            if score > 0.0 {
                scored_docs.push(SearchResult {
                    chunk_id: doc_id.clone(),
                    score,
                    content: content.clone(),
                    metadata: HashMap::new(),
                });
            }
        }

        // Sort by score descending
        scored_docs.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored_docs.into_iter().take(top_k).collect()
    }

    fn calculate_bm25_score(&self, query_terms: &[String], document: &str) -> f32 {
        let doc_terms = self.tokenize(document);
        let doc_length = doc_terms.len() as f32;

        // Count term frequencies in document
        let mut term_freq = HashMap::new();
        for term in &doc_terms {
            *term_freq.entry(term.clone()).or_insert(0) += 1;
        }

        let mut score = 0.0;

        for query_term in query_terms {
            let tf = *term_freq.get(query_term).unwrap_or(&0) as f32;

            if tf > 0.0 {
                // IDF calculation: log((N - df + 0.5) / (df + 0.5))
                let df = *self.term_doc_freq.get(query_term).unwrap_or(&0) as f32;
                let idf = ((self.total_docs as f32 - df + 0.5) / (df + 0.5)).ln();

                // BM25 formula
                let tf_component = (tf * (self.k1 + 1.0)) /
                    (tf + self.k1 * (1.0 - self.b + self.b * doc_length / self.avg_doc_length));

                score += idf * tf_component;
            }
        }

        score
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split_whitespace()
            .map(|word| {
                // Remove punctuation but keep underscores for code terms like "uvm_config_db"
                word.chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_')
                    .collect::<String>()
            })
            .filter(|word| !word.is_empty() && word.len() > 2) // Filter very short words
            .collect()
    }

    /// Enhanced tokenization for UVM/SystemVerilog code
    pub fn tokenize_code_aware(&self, text: &str) -> Vec<String> {
        let mut tokens = Vec::new();

        // First get standard tokens
        let standard_tokens = self.tokenize(text);
        tokens.extend(standard_tokens);

        // Add special patterns for UVM/SystemVerilog
        let uvm_patterns = vec![
            r"uvm_\w+",           // uvm_config_db, uvm_object, etc.
            r"\w+_phase",         // build_phase, run_phase, etc.
            r"`uvm_\w+",          // `uvm_component_utils, etc.
            r"\w+_imp\b",         // analysis_imp, etc.
            r"\w+_export\b",      // analysis_export, etc.
        ];

        for pattern in &uvm_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                for mat in regex.find_iter(text) {
                    tokens.push(mat.as_str().to_lowercase());
                }
            }
        }

        // Remove duplicates while preserving order
        let mut seen = HashSet::new();
        tokens.into_iter()
            .filter(|token| seen.insert(token.clone()))
            .collect()
    }
}

/// Advanced hybrid search that combines semantic and keyword matching
pub struct HybridSearch {
    bm25: BM25Search,
    semantic_weight: f32,
    keyword_weight: f32,
}

impl HybridSearch {
    pub fn new(semantic_weight: f32, keyword_weight: f32) -> Self {
        Self {
            bm25: BM25Search::new(),
            semantic_weight,
            keyword_weight,
        }
    }

    /// Index a document for both semantic and keyword search
    pub fn index_document(&mut self, doc_id: &str, content: &str) {
        self.bm25.index_document(doc_id, content);
    }

    /// Perform hybrid search combining semantic and keyword results
    pub fn search(
        &self,
        query: &str,
        semantic_results: Vec<SearchResult>,
        documents: &[(String, String)],
        top_k: usize,
    ) -> Vec<SearchResult> {
        // Get BM25 keyword results
        let keyword_results = self.bm25.search(query, documents, top_k * 2);

        // Merge results using Reciprocal Rank Fusion (RRF)
        self.merge_with_rrf(semantic_results, keyword_results, top_k)
    }

    /// Merge semantic and keyword results using Reciprocal Rank Fusion
    fn merge_with_rrf(
        &self,
        semantic_results: Vec<SearchResult>,
        keyword_results: Vec<SearchResult>,
        top_k: usize,
    ) -> Vec<SearchResult> {
        const RRF_K: f32 = 60.0; // Standard RRF parameter

        let mut doc_scores: HashMap<String, (f32, String, HashMap<String, String>)> = HashMap::new();

        // Add semantic scores
        for (rank, result) in semantic_results.iter().enumerate() {
            let rrf_score = 1.0 / (RRF_K + rank as f32 + 1.0);
            doc_scores.insert(
                result.chunk_id.clone(),
                (
                    self.semantic_weight * rrf_score,
                    result.content.clone(),
                    result.metadata.clone(),
                ),
            );
        }

        // Add keyword scores
        for (rank, result) in keyword_results.iter().enumerate() {
            let rrf_score = 1.0 / (RRF_K + rank as f32 + 1.0);
            doc_scores
                .entry(result.chunk_id.clone())
                .and_modify(|(score, content, metadata)| {
                    *score += self.keyword_weight * rrf_score;
                })
                .or_insert((
                    self.keyword_weight * rrf_score,
                    result.content.clone(),
                    result.metadata.clone(),
                ));
        }

        // Convert to results and sort
        let mut final_results: Vec<SearchResult> = doc_scores
            .into_iter()
            .map(|(chunk_id, (score, content, metadata))| SearchResult {
                chunk_id,
                score,
                content,
                metadata,
            })
            .collect();

        final_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        final_results.into_iter().take(top_k).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bm25_uvm_search() {
        let mut bm25 = BM25Search::new();

        // Index some test documents
        let docs = vec![
            ("doc1", "uvm_config_db is used to store configuration objects in UVM testbenches"),
            ("doc2", "The build_phase is where uvm_config_db get and set operations typically occur"),
            ("doc3", "SystemVerilog classes extend uvm_object for proper UVM functionality"),
        ];

        for (id, content) in &docs {
            bm25.index_document(id, content);
        }

        // Search for UVM config database
        let results = bm25.search("uvm_config_db", &docs.iter().map(|(id, content)| (id.to_string(), content.to_string())).collect::<Vec<_>>(), 5);

        assert!(!results.is_empty());
        assert!(results[0].chunk_id == "doc1" || results[0].chunk_id == "doc2");
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn test_tokenization() {
        let bm25 = BM25Search::new();
        let tokens = bm25.tokenize("uvm_config_db::get() is a method");
        assert!(tokens.contains(&"uvm_config_db".to_string()));
        assert!(tokens.contains(&"get".to_string()));
        assert!(tokens.contains(&"method".to_string()));
    }
}