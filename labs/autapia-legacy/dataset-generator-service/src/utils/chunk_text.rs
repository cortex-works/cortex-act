use anyhow::{Result, anyhow};
use regex::Regex;

pub fn chunk_text(
    text: &str,
    chunk_size: usize,
    chunk_overlap: usize,
    strategy: &str,
) -> Result<Vec<String>> {
    if chunk_size == 0 {
        return Err(anyhow!("Chunk size must be greater than 0"));
    }

    if chunk_overlap >= chunk_size {
        return Err(anyhow!("Chunk overlap must be less than chunk size"));
    }

    match strategy {
        "sentence" => chunk_by_sentences(text, chunk_size, chunk_overlap),
        "paragraph" => chunk_by_paragraphs(text, chunk_size, chunk_overlap),
        "fixed" => chunk_by_fixed_size(text, chunk_size, chunk_overlap),
        _ => Err(anyhow!("Unknown chunking strategy: {}", strategy)),
    }
}

fn chunk_by_sentences(text: &str, chunk_size: usize, chunk_overlap: usize) -> Result<Vec<String>> {
    let sentences = split_into_sentences(text);
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_size = 0;

    for sentence in sentences {
        let sentence_len = sentence.chars().count();
        
        // If adding this sentence would exceed chunk size, finalize current chunk
        if current_size + sentence_len > chunk_size && !current_chunk.is_empty() {
            chunks.push(current_chunk.trim().to_string());
            
            // Start new chunk with overlap
            current_chunk = if chunk_overlap > 0 {
                get_overlap_text(&current_chunk, chunk_overlap)
            } else {
                String::new()
            };
            current_size = current_chunk.chars().count();
        }
        
        // Add sentence to current chunk
        if !current_chunk.is_empty() {
            current_chunk.push(' ');
        }
        current_chunk.push_str(&sentence);
        current_size += sentence_len + if current_chunk.len() > sentence_len { 1 } else { 0 };
    }

    // Add final chunk if not empty
    if !current_chunk.trim().is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    Ok(chunks)
}

fn chunk_by_paragraphs(text: &str, chunk_size: usize, chunk_overlap: usize) -> Result<Vec<String>> {
    let paragraphs: Vec<&str> = text.split("\n\n").filter(|p| !p.trim().is_empty()).collect();
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_size = 0;

    for paragraph in paragraphs {
        let paragraph_len = paragraph.chars().count();
        
        // If adding this paragraph would exceed chunk size, finalize current chunk
        if current_size + paragraph_len > chunk_size && !current_chunk.is_empty() {
            chunks.push(current_chunk.trim().to_string());
            
            // Start new chunk with overlap
            current_chunk = if chunk_overlap > 0 {
                get_overlap_text(&current_chunk, chunk_overlap)
            } else {
                String::new()
            };
            current_size = current_chunk.chars().count();
        }
        
        // Add paragraph to current chunk
        if !current_chunk.is_empty() {
            current_chunk.push_str("\n\n");
            current_size += 2;
        }
        current_chunk.push_str(paragraph);
        current_size += paragraph_len;
    }

    // Add final chunk if not empty
    if !current_chunk.trim().is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    Ok(chunks)
}

fn chunk_by_fixed_size(text: &str, chunk_size: usize, chunk_overlap: usize) -> Result<Vec<String>> {
    let chars: Vec<char> = text.chars().collect();
    let mut chunks = Vec::new();
    let mut start = 0;

    while start < chars.len() {
        let end = (start + chunk_size).min(chars.len());
        let chunk: String = chars[start..end].iter().collect();
        
        if !chunk.trim().is_empty() {
            chunks.push(chunk.trim().to_string());
        }
        
        // Move start position considering overlap
        start = if end == chars.len() {
            chars.len() // End of text
        } else {
            (start + chunk_size).saturating_sub(chunk_overlap)
        };
    }

    Ok(chunks)
}

fn split_into_sentences(text: &str) -> Vec<String> {
    // Simple sentence splitting using regex
    let sentence_regex = Regex::new(r"[.!?]+\s+").unwrap();
    let mut sentences = Vec::new();
    let mut last_end = 0;

    for mat in sentence_regex.find_iter(text) {
        let sentence = text[last_end..mat.end()].trim().to_string();
        if !sentence.is_empty() {
            sentences.push(sentence);
        }
        last_end = mat.end();
    }

    // Add remaining text as final sentence
    if last_end < text.len() {
        let final_sentence = text[last_end..].trim().to_string();
        if !final_sentence.is_empty() {
            sentences.push(final_sentence);
        }
    }

    // If no sentences found, return the whole text
    if sentences.is_empty() && !text.trim().is_empty() {
        sentences.push(text.trim().to_string());
    }

    sentences
}

fn get_overlap_text(text: &str, overlap_size: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= overlap_size {
        return text.to_string();
    }

    let start = chars.len() - overlap_size;
    chars[start..].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_by_fixed_size() {
        let text = "This is a test text for chunking.";
        let chunks = chunk_by_fixed_size(text, 10, 2).unwrap();
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_chunk_by_sentences() {
        let text = "First sentence. Second sentence! Third sentence?";
        let chunks = chunk_by_sentences(text, 20, 5).unwrap();
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_chunk_by_paragraphs() {
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let chunks = chunk_by_paragraphs(text, 30, 5).unwrap();
        assert!(!chunks.is_empty());
    }
} 