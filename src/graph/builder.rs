use crate::chunker::Chunk;
use anyhow::Result;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub node_type: NodeType,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Chunk,
    Word,
    Chapter,
    Document,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeType {
    Similarity,
    Contains,
    PartOf,
    Sequential,
    Reference,
}

pub struct GraphBuilder {
    nodes: HashMap<String, GraphNode>,
    edges: Vec<GraphEdge>,
    similarity_threshold: f32,
}

impl GraphBuilder {
    pub fn new(similarity_threshold: f32) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            similarity_threshold,
        }
    }

    pub fn build_relationships(&mut self, chunks: &[Chunk]) -> Result<()> {
        // Add chunk nodes
        for chunk in chunks {
            self.add_chunk_node(chunk);
        }

        // Build chunk-to-chunk similarity relationships
        self.build_similarity_edges(chunks)?;

        // Extract and add word nodes
        self.extract_word_nodes(chunks);

        // Build hierarchical relationships
        self.build_hierarchical_relationships(chunks);

        // Build sequential relationships
        self.build_sequential_relationships(chunks);

        Ok(())
    }

    fn add_chunk_node(&mut self, chunk: &Chunk) {
        let mut metadata = HashMap::new();
        metadata.insert("source_file".to_string(), chunk.metadata.source_file.clone());
        metadata.insert("chunk_type".to_string(), format!("{:?}", chunk.metadata.chunk_type));

        if let Some(chapter) = &chunk.metadata.chapter {
            metadata.insert("chapter".to_string(), chapter.clone());
        }

        if let Some(section) = &chunk.metadata.section {
            metadata.insert("section".to_string(), section.clone());
        }

        let node = GraphNode {
            id: chunk.id.clone(),
            node_type: NodeType::Chunk,
            content: chunk.content.clone(),
            metadata,
        };

        self.nodes.insert(chunk.id.clone(), node);
    }

    fn build_similarity_edges(&mut self, chunks: &[Chunk]) -> Result<()> {
        for i in 0..chunks.len() {
            for j in i + 1..chunks.len() {
                let similarity = self.calculate_similarity(&chunks[i], &chunks[j]);

                if similarity > self.similarity_threshold {
                    let edge = GraphEdge {
                        from: chunks[i].id.clone(),
                        to: chunks[j].id.clone(),
                        edge_type: EdgeType::Similarity,
                        weight: similarity,
                    };
                    self.edges.push(edge);
                }
            }
        }
        Ok(())
    }

    fn calculate_similarity(&self, chunk1: &Chunk, chunk2: &Chunk) -> f32 {
        // Simple cosine similarity if embeddings are available
        if !chunk1.embedding.is_empty() && !chunk2.embedding.is_empty() {
            self.cosine_similarity(&chunk1.embedding, &chunk2.embedding)
        } else {
            // Fallback to simple text similarity
            self.jaccard_similarity(&chunk1.content, &chunk2.content)
        }
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

    fn jaccard_similarity(&self, text1: &str, text2: &str) -> f32 {
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

    fn extract_word_nodes(&mut self, chunks: &[Chunk]) {
        let mut word_frequencies = HashMap::new();

        // Extract keywords/important words
        for chunk in chunks {
            let words = self.extract_keywords(&chunk.content);
            for word in words {
                *word_frequencies.entry(word.clone()).or_insert(0) += 1;

                // Add word node if it doesn't exist
                if !self.nodes.contains_key(&word) {
                    let node = GraphNode {
                        id: word.clone(),
                        node_type: NodeType::Word,
                        content: word.clone(),
                        metadata: HashMap::new(),
                    };
                    self.nodes.insert(word.clone(), node);
                }

                // Add contains edge from chunk to word
                let edge = GraphEdge {
                    from: chunk.id.clone(),
                    to: word,
                    edge_type: EdgeType::Contains,
                    weight: 1.0,
                };
                self.edges.push(edge);
            }
        }
    }

    fn extract_keywords(&self, text: &str) -> Vec<String> {
        // Simple keyword extraction - filter out common words and short words
        let stop_words = ["the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by"];

        text.split_whitespace()
            .map(|w| w.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|w| w.len() > 3 && !stop_words.contains(&w.as_str()))
            .collect()
    }

    fn build_hierarchical_relationships(&mut self, chunks: &[Chunk]) {
        // Group chunks by document and chapter
        let mut documents = HashMap::new();
        let mut chapters = HashMap::new();

        for chunk in chunks {
            let doc_id = chunk.metadata.source_file.clone();
            documents.entry(doc_id.clone()).or_insert_with(Vec::new).push(chunk.id.clone());

            if let Some(chapter) = &chunk.metadata.chapter {
                let chapter_id = format!("{}#{}", doc_id, chapter);
                chapters.entry(chapter_id.clone()).or_insert_with(Vec::new).push(chunk.id.clone());

                // Add chapter node if it doesn't exist
                if !self.nodes.contains_key(&chapter_id) {
                    let node = GraphNode {
                        id: chapter_id.clone(),
                        node_type: NodeType::Chapter,
                        content: chapter.clone(),
                        metadata: HashMap::new(),
                    };
                    self.nodes.insert(chapter_id.clone(), node);
                }

                // Add part-of edge from chunk to chapter
                let edge = GraphEdge {
                    from: chunk.id.clone(),
                    to: chapter_id,
                    edge_type: EdgeType::PartOf,
                    weight: 1.0,
                };
                self.edges.push(edge);
            }
        }

        // Add document nodes and edges
        for (doc_id, chunk_ids) in documents {
            if !self.nodes.contains_key(&doc_id) {
                let node = GraphNode {
                    id: doc_id.clone(),
                    node_type: NodeType::Document,
                    content: doc_id.clone(),
                    metadata: HashMap::new(),
                };
                self.nodes.insert(doc_id.clone(), node);
            }

            for chunk_id in chunk_ids {
                let edge = GraphEdge {
                    from: chunk_id,
                    to: doc_id.clone(),
                    edge_type: EdgeType::PartOf,
                    weight: 1.0,
                };
                self.edges.push(edge);
            }
        }
    }

    fn build_sequential_relationships(&mut self, chunks: &[Chunk]) {
        // Group chunks by source file and sort by position
        let mut file_chunks: HashMap<String, Vec<&Chunk>> = HashMap::new();

        for chunk in chunks {
            file_chunks.entry(chunk.metadata.source_file.clone())
                       .or_insert_with(Vec::new)
                       .push(chunk);
        }

        // Sort chunks by their start position and create sequential edges
        for (_, mut chunks) in file_chunks {
            chunks.sort_by_key(|c| c.boundaries.0);

            for i in 0..chunks.len() - 1 {
                let edge = GraphEdge {
                    from: chunks[i].id.clone(),
                    to: chunks[i + 1].id.clone(),
                    edge_type: EdgeType::Sequential,
                    weight: 1.0,
                };
                self.edges.push(edge);
            }
        }
    }

    pub fn find_related_chunks(&self, chunk_id: &str, max_depth: usize) -> Vec<String> {
        let mut related = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        queue.push_back((chunk_id.to_string(), 0));
        visited.insert(chunk_id.to_string());

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            for edge in &self.edges {
                let next_id = if edge.from == current_id {
                    &edge.to
                } else if edge.to == current_id {
                    &edge.from
                } else {
                    continue;
                };

                if !visited.contains(next_id) {
                    visited.insert(next_id.clone());

                    if let Some(node) = self.nodes.get(next_id) {
                        if matches!(node.node_type, NodeType::Chunk) {
                            related.push(next_id.clone());
                            queue.push_back((next_id.clone(), depth + 1));
                        }
                    }
                }
            }
        }

        related
    }

    pub fn get_nodes(&self) -> &HashMap<String, GraphNode> {
        &self.nodes
    }

    pub fn get_edges(&self) -> &[GraphEdge] {
        &self.edges
    }
}