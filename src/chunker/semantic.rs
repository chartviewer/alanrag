use anyhow::Result;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use sha2::{Sha256, Digest};

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
    pub file_hash: Option<String>,        // SHA256 hash of source file
    pub timestamp: chrono::DateTime<chrono::Utc>,  // When the chunk was created
    pub line_start: usize,                // Starting line number in source file
    pub line_end: usize,                  // Ending line number in source file
    pub tags: Vec<String>,                 // Searchable tags
    pub dependencies: Vec<String>,        // For code: imported modules/packages
    pub chunk_size: usize,                // Size of chunk in bytes
    pub parent_chunk_id: Option<String>,  // For hierarchical chunking
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
        // Calculate file hash for metadata
        let file_hash = Self::calculate_file_hash(text);
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
                            file_hash: Some(file_hash.clone()),
                            timestamp: Utc::now(),
                            line_start: start_pos,
                            line_end: current_pos,
                            tags: Self::extract_tags(&current_chunk),
                            dependencies: vec![],
                            chunk_size: current_chunk.len(),
                            parent_chunk_id: None,
                        },
                        boundaries: (start_pos, current_pos),
                    };
                    chunks.push(chunk);
                }

                // Start new chunk with overlap (Unicode-safe)
                let overlap_chars = self.overlap_tokens;
                let chars: Vec<char> = current_chunk.chars().collect();
                let overlap_start_chars = chars.len().saturating_sub(overlap_chars);

                // Use character-based slicing instead of byte-based
                current_chunk = chars[overlap_start_chars..].iter().collect::<String>();

                // Calculate position based on character boundaries
                let chars_before_overlap = chars.len() - current_chunk.chars().count();
                start_pos = current_pos - (chars.len() - chars_before_overlap);
            }

            current_chunk.push_str(&sentence);
            current_pos += sentence.chars().count(); // Use character count instead of byte length
        }

        // Add final chunk
        if !current_chunk.is_empty() && current_chunk.len() >= self.min_chunk_size {
            let chunk = Chunk {
                id: Uuid::new_v4().to_string(),
                content: current_chunk.clone(),
                embedding: vec![],
                metadata: ChunkMetadata {
                    source_file: source_file.to_string(),
                    chunk_type: ChunkType::Text,
                    chapter: None,
                    section: None,
                    language: None,
                    file_hash: Some(file_hash),
                    timestamp: Utc::now(),
                    line_start: start_pos,
                    line_end: current_pos,
                    tags: Self::extract_tags(&current_chunk),
                    dependencies: vec![],
                    chunk_size: current_chunk.len(),
                    parent_chunk_id: None,
                },
                boundaries: (start_pos, current_pos),
            };
            chunks.push(chunk);
        }

        Ok(chunks)
    }

    pub fn chunk_code(&self, code: &str, language: &str, source_file: &str) -> Result<Vec<Chunk>> {
        // Calculate file hash for metadata
        let file_hash = Self::calculate_file_hash(code);

        // Enhanced code chunking with better structure awareness
        let lines: Vec<&str> = code.lines().collect();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut start_line = 0;
        let mut brace_depth: i32 = 0;
        let mut in_function = false;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();

            // Track brace depth for better boundaries
            for ch in line.chars() {
                match ch {
                    '{' | '(' | '[' => brace_depth += 1,
                    '}' | ')' | ']' => brace_depth = brace_depth.saturating_sub(1),
                    _ => {}
                }
            }

            // Detect function/class boundaries with language-specific patterns
            let is_function_start = match language {
                "rust" => trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") ||
                          trimmed.starts_with("impl ") || trimmed.starts_with("pub struct ") ||
                          trimmed.starts_with("struct ") || trimmed.starts_with("enum ") ||
                          trimmed.starts_with("pub enum ") || trimmed.starts_with("trait ") ||
                          trimmed.starts_with("pub trait "),
                "python" => trimmed.starts_with("def ") || trimmed.starts_with("class ") ||
                            trimmed.starts_with("async def "),
                "javascript" | "typescript" => trimmed.starts_with("function ") ||
                                              trimmed.starts_with("class ") ||
                                              trimmed.starts_with("const ") && trimmed.contains(" = ") ||
                                              trimmed.starts_with("export function ") ||
                                              trimmed.starts_with("export class "),
                "java" => trimmed.starts_with("public class ") || trimmed.starts_with("class ") ||
                         trimmed.starts_with("public static ") || trimmed.starts_with("private ") ||
                         trimmed.starts_with("protected "),
                "go" => trimmed.starts_with("func ") || trimmed.starts_with("type ") ||
                       trimmed.starts_with("struct "),
                _ => trimmed.starts_with("fn ") || trimmed.starts_with("def ") ||
                    trimmed.starts_with("function ") || trimmed.starts_with("class ")
            };

            // Decide whether to start a new chunk
            let should_split = is_function_start && !current_chunk.is_empty() &&
                              (brace_depth == 0 || (language == "python" && !in_function));

            if should_split {
                // Save current chunk if it meets minimum size
                if current_chunk.len() >= self.min_chunk_size {
                    let chunk = Chunk {
                        id: Uuid::new_v4().to_string(),
                        content: current_chunk.trim().to_string(),
                        embedding: vec![],
                        metadata: ChunkMetadata {
                            source_file: source_file.to_string(),
                            chunk_type: ChunkType::Code,
                            chapter: None,
                            section: Self::extract_function_name(&current_chunk),
                            language: Some(language.to_string()),
                            file_hash: Some(file_hash.clone()),
                            timestamp: Utc::now(),
                            line_start: start_line,
                            line_end: i,
                            tags: Self::extract_code_tags(&current_chunk, language),
                            dependencies: Self::extract_dependencies(&current_chunk, language),
                            chunk_size: current_chunk.len(),
                            parent_chunk_id: None,
                        },
                        boundaries: (start_line, i),
                    };
                    chunks.push(chunk);
                }
                current_chunk.clear();
                start_line = i;
                in_function = is_function_start;
            } else if is_function_start {
                in_function = true;
            }

            current_chunk.push_str(line);
            current_chunk.push('\n');

            // Split if chunk gets too large, but try to respect boundaries
            if current_chunk.len() > self.max_chunk_size && brace_depth == 0 {
                if current_chunk.len() >= self.min_chunk_size {
                    let chunk = Chunk {
                        id: Uuid::new_v4().to_string(),
                        content: current_chunk.trim().to_string(),
                        embedding: vec![],
                        metadata: ChunkMetadata {
                            source_file: source_file.to_string(),
                            chunk_type: ChunkType::Code,
                            chapter: None,
                            section: Self::extract_function_name(&current_chunk),
                            language: Some(language.to_string()),
                            file_hash: Some(file_hash.clone()),
                            timestamp: Utc::now(),
                            line_start: start_line,
                            line_end: i + 1,
                            tags: Self::extract_code_tags(&current_chunk, language),
                            dependencies: Self::extract_dependencies(&current_chunk, language),
                            chunk_size: current_chunk.len(),
                            parent_chunk_id: None,
                        },
                        boundaries: (start_line, i + 1),
                    };
                    chunks.push(chunk);
                }
                current_chunk.clear();
                start_line = i + 1;
                in_function = false;
            }
        }

        // Add final chunk
        if !current_chunk.trim().is_empty() && current_chunk.len() >= self.min_chunk_size {
            let chunk = Chunk {
                id: Uuid::new_v4().to_string(),
                content: current_chunk.trim().to_string(),
                embedding: vec![],
                metadata: ChunkMetadata {
                    source_file: source_file.to_string(),
                    chunk_type: ChunkType::Code,
                    chapter: None,
                    section: Self::extract_function_name(&current_chunk),
                    language: Some(language.to_string()),
                    file_hash: Some(file_hash),
                    timestamp: Utc::now(),
                    line_start: start_line,
                    line_end: lines.len(),
                    tags: Self::extract_code_tags(&current_chunk, language),
                    dependencies: Self::extract_dependencies(&current_chunk, language),
                    chunk_size: current_chunk.len(),
                    parent_chunk_id: None,
                },
                boundaries: (start_line, lines.len()),
            };
            chunks.push(chunk);
        }

        Ok(chunks)
    }

    fn extract_function_name(code: &str) -> Option<String> {
        // Extract the function/class name from the code chunk (Unicode-safe)
        for line in code.lines() {
            let trimmed = line.trim();

            // Rust - use safe substring extraction
            if let Some(pos) = trimmed.find("fn ") {
                if let Some(name_part) = Self::safe_substring(trimmed, pos + 3, None) {
                    if let Some(end) = name_part.find(|c: char| c == '(' || c == '<') {
                        if let Some(name) = Self::safe_substring(&name_part, 0, Some(end)) {
                            return Some(name.trim().to_string());
                        }
                    }
                }
            }

            // Python
            if let Some(pos) = trimmed.find("def ") {
                if let Some(name_part) = Self::safe_substring(trimmed, pos + 4, None) {
                    if let Some(end) = name_part.find('(') {
                        if let Some(name) = Self::safe_substring(&name_part, 0, Some(end)) {
                            return Some(name.trim().to_string());
                        }
                    }
                }
            }

            // Class definitions
            if let Some(pos) = trimmed.find("class ") {
                if let Some(name_part) = Self::safe_substring(trimmed, pos + 6, None) {
                    if let Some(end) = name_part.find(|c: char| c == ' ' || c == '{' || c == '(' || c == ':' || c == '<') {
                        if let Some(name) = Self::safe_substring(&name_part, 0, Some(end)) {
                            return Some(name.trim().to_string());
                        }
                    }
                }
            }
        }
        None
    }

    /// Unicode-safe substring extraction using character indices
    fn safe_substring(s: &str, start_chars: usize, end_chars: Option<usize>) -> Option<String> {
        let chars: Vec<char> = s.chars().collect();

        if start_chars >= chars.len() {
            return None;
        }

        let end = end_chars.unwrap_or(chars.len()).min(chars.len());

        if start_chars >= end {
            return None;
        }

        Some(chars[start_chars..end].iter().collect())
    }

    fn split_sentences(&self, text: &str) -> Vec<String> {
        // Improved sentence splitting with proper boundary detection
        let mut sentences = Vec::new();
        let mut current_sentence = String::new();
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            current_sentence.push(ch);

            // Check for sentence endings
            if ch == '.' || ch == '!' || ch == '?' {
                // Look ahead to see if this is really a sentence boundary
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == ' ' || next_ch == '\n' || next_ch == '\t' {
                        // This is likely a sentence boundary
                        // Include the space after the punctuation
                        if next_ch == ' ' {
                            current_sentence.push(chars.next().unwrap());
                        }

                        sentences.push(current_sentence.clone());
                        current_sentence.clear();
                    }
                } else {
                    // End of text
                    sentences.push(current_sentence.clone());
                    current_sentence.clear();
                }
            }
        }

        // Add any remaining text as a final sentence
        if !current_sentence.trim().is_empty() {
            sentences.push(current_sentence);
        }

        // Filter out empty sentences
        sentences.into_iter()
            .filter(|s| !s.trim().is_empty())
            .collect()
    }

    fn calculate_file_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn extract_tags(text: &str) -> Vec<String> {
        // Extract important keywords as tags
        let mut tags = Vec::new();

        // Common technical terms that might be useful as tags
        let keywords = ["TODO", "FIXME", "NOTE", "WARNING", "IMPORTANT",
                       "API", "REST", "GraphQL", "Database", "Config"];

        for keyword in &keywords {
            if text.contains(keyword) {
                tags.push(keyword.to_lowercase());
            }
        }

        tags
    }

    fn extract_code_tags(code: &str, language: &str) -> Vec<String> {
        let mut tags = Self::extract_tags(code);

        // Add language as a tag
        tags.push(language.to_string());

        // Add common patterns
        if code.contains("async ") || code.contains("await ") {
            tags.push("async".to_string());
        }
        if code.contains("test") || code.contains("Test") {
            tags.push("test".to_string());
        }

        tags
    }

    fn extract_dependencies(code: &str, language: &str) -> Vec<String> {
        let mut deps = Vec::new();

        for line in code.lines() {
            let trimmed = line.trim();

            match language {
                "rust" => {
                    if trimmed.starts_with("use ") {
                        if let Some(dep) = trimmed.strip_prefix("use ") {
                            if let Some(end) = dep.find(':') {
                                deps.push(dep[..end].to_string());
                            } else if let Some(end) = dep.find(';') {
                                deps.push(dep[..end].to_string());
                            }
                        }
                    }
                },
                "python" => {
                    if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                        deps.push(trimmed.to_string());
                    }
                },
                "javascript" | "typescript" => {
                    if trimmed.starts_with("import ") || trimmed.starts_with("const ") && trimmed.contains("require(") {
                        deps.push(trimmed.to_string());
                    }
                },
                _ => {}
            }
        }

        deps.dedup();
        deps
    }
}