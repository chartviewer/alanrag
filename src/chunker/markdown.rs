use super::{Chunk, ChunkMetadata, ChunkType};
use anyhow::Result;
use pulldown_cmark::{Parser, Event, Tag, TagEnd, HeadingLevel};
use uuid::Uuid;

pub struct MarkdownProcessor;

#[derive(Debug, Clone)]
struct HeaderInfo {
    text: String,
    level: u32,
}

impl MarkdownProcessor {
    pub fn extract_and_chunk(content: &str, file_path: &str, chunker: &super::SemanticChunker) -> Result<Vec<Chunk>> {
        let mut sections = Vec::new();
        let mut current_section = String::new();
        let mut header_stack: Vec<HeaderInfo> = Vec::new();

        let parser = Parser::new(content);
        let mut in_heading = false;
        let mut heading_text = String::new();
        let mut heading_level = 1;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    if !current_section.is_empty() {
                        sections.push((header_stack.clone(), current_section.clone()));
                        current_section.clear();
                    }
                    in_heading = true;
                    heading_text.clear();
                    heading_level = level as u32;
                }
                Event::End(TagEnd::Heading(_)) => {
                    in_heading = false;

                    // Update header stack based on level
                    // Remove headers at same or deeper level
                    header_stack.retain(|h| h.level < heading_level);

                    // Add current header
                    header_stack.push(HeaderInfo {
                        text: heading_text.clone(),
                        level: heading_level,
                    });
                }
                Event::Text(text) => {
                    if in_heading {
                        heading_text.push_str(&text);
                    } else {
                        current_section.push_str(&text);
                    }
                }
                Event::SoftBreak | Event::HardBreak => {
                    if !in_heading {
                        current_section.push('\n');
                    }
                }
                Event::Code(text) => {
                    if in_heading {
                        heading_text.push_str(&text);
                    } else {
                        current_section.push_str(&text);
                    }
                }
                _ => {}
            }
        }

        // Add final section
        if !current_section.is_empty() {
            sections.push((header_stack, current_section));
        }

        let mut all_chunks = Vec::new();

        for (headers, section_content) in sections {
            let mut chunks = chunker.chunk_text(&section_content, file_path)?;

            // Extract chapter and section information from header stack
            let (chapter, section) = Self::extract_chapter_and_section(&headers);

            // Update metadata
            for chunk in &mut chunks {
                chunk.metadata.chunk_type = ChunkType::Markdown;
                chunk.metadata.chapter = chapter.clone();
                chunk.metadata.section = section.clone();
            }

            all_chunks.extend(chunks);
        }

        Ok(all_chunks)
    }

    fn extract_chapter_and_section(headers: &[HeaderInfo]) -> (Option<String>, Option<String>) {
        if headers.is_empty() {
            return (None, None);
        }

        // Find the main chapter (typically level 1 or 2 headings that mention "Chapter")
        let mut chapter = None;
        let mut section = None;

        for header in headers {
            let header_text = &header.text;

            // Check if this looks like a chapter
            if (header.level <= 2 && (
                header_text.to_lowercase().contains("chapter") ||
                header_text.to_lowercase().starts_with("chapter ") ||
                // Match numbered chapters like "4.3 The uvm_object Class"
                Self::is_numbered_section(header_text)
            )) || (header.level == 1) {
                chapter = Some(header_text.clone());
            }

            // The most specific (deepest) header becomes the section
            section = Some(header_text.clone());
        }

        // If we found a chapter-like header, use it as chapter
        // Otherwise, use the top-level header as chapter if it exists
        if chapter.is_none() && !headers.is_empty() {
            chapter = Some(headers[0].text.clone());
        }

        (chapter, section)
    }

    fn is_numbered_section(text: &str) -> bool {
        // Match patterns like "4.3 Something", "Chapter 4", etc.
        let text = text.trim();

        // Check for patterns like "4.3", "1.2.3", etc. at the start
        let words: Vec<&str> = text.split_whitespace().collect();
        if let Some(first_word) = words.first() {
            // Match patterns like "4.3", "1.2", "Chapter", etc.
            if first_word.contains('.') && first_word.chars().any(|c| c.is_ascii_digit()) {
                return true;
            }
            if first_word.parse::<f64>().is_ok() {
                return true;
            }
        }

        false
    }
}