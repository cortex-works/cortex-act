use crate::dataset_generator::{QualityFilters, FilteringConfig};
use crate::types::{ProcessedChunk, DatasetSample};
use anyhow::Result;
use std::collections::HashSet;

pub fn apply_basic_filters(
    chunks: Vec<ProcessedChunk>,
    quality_filters: &QualityFilters,
) -> Result<Vec<ProcessedChunk>> {
    let mut filtered_chunks = Vec::new();
    let mut seen_texts = HashSet::new();

    for chunk in chunks {
        // Check text length
        let text_len = chunk.text.chars().count();
        if text_len < quality_filters.min_text_length as usize {
            continue;
        }
        if text_len > quality_filters.max_text_length as usize {
            continue;
        }

        // Check for duplicates if enabled
        if quality_filters.remove_duplicates {
            let normalized_text = normalize_text(&chunk.text);
            if seen_texts.contains(&normalized_text) {
                continue;
            }
            seen_texts.insert(normalized_text);
        }

        // Calculate basic quality score
        let quality_score = calculate_text_quality(&chunk.text);
        if quality_score >= quality_filters.min_quality_score {
            let mut filtered_chunk = chunk;
            filtered_chunk.quality_score = quality_score;
            filtered_chunks.push(filtered_chunk);
        }
    }

    Ok(filtered_chunks)
}

pub fn apply_quality_filters(
    samples: Vec<DatasetSample>,
    filtering_config: &FilteringConfig,
) -> Result<Vec<DatasetSample>> {
    let mut filtered_samples = samples;

    // Apply quality filters
    if let Some(quality_filters) = &filtering_config.quality {
        filtered_samples = apply_quality_filters_to_samples(filtered_samples, quality_filters)?;
    }

    // Apply content filters
    if let Some(content_filters) = &filtering_config.content {
        filtered_samples = apply_content_filters(filtered_samples, content_filters)?;
    }

    // Apply diversity filters
    if let Some(diversity_filters) = &filtering_config.diversity {
        filtered_samples = apply_diversity_filters(filtered_samples, diversity_filters)?;
    }

    Ok(filtered_samples)
}

fn apply_quality_filters_to_samples(
    samples: Vec<DatasetSample>,
    quality_filters: &QualityFilters,
) -> Result<Vec<DatasetSample>> {
    let mut filtered_samples = Vec::new();
    let mut seen_inputs = HashSet::new();

    for sample in samples {
        // Check input length
        let input_len = sample.input.chars().count();
        if input_len < quality_filters.min_text_length as usize {
            continue;
        }
        if input_len > quality_filters.max_text_length as usize {
            continue;
        }

        // Check output length
        let output_len = sample.output.chars().count();
        if output_len < quality_filters.min_text_length as usize {
            continue;
        }
        if output_len > quality_filters.max_text_length as usize {
            continue;
        }

        // Check for duplicates if enabled
        if quality_filters.remove_duplicates {
            let normalized_input = normalize_text(&sample.input);
            if seen_inputs.contains(&normalized_input) {
                continue;
            }
            seen_inputs.insert(normalized_input);
        }

        // Check quality score
        if sample.quality_score >= quality_filters.min_quality_score {
            filtered_samples.push(sample);
        }
    }

    Ok(filtered_samples)
}

fn apply_content_filters(
    samples: Vec<DatasetSample>,
    content_filters: &crate::dataset_generator::ContentFilters,
) -> Result<Vec<DatasetSample>> {
    let mut filtered_samples = Vec::new();

    for sample in samples {
        let combined_text = format!("{} {}", sample.input, sample.output);
        
        // Check exclude patterns
        let mut should_exclude = false;
        for pattern in &content_filters.exclude_patterns {
            if combined_text.contains(pattern) {
                should_exclude = true;
                break;
            }
        }
        if should_exclude {
            continue;
        }

        // Check required patterns
        if !content_filters.required_patterns.is_empty() {
            let mut has_required = false;
            for pattern in &content_filters.required_patterns {
                if combined_text.contains(pattern) {
                    has_required = true;
                    break;
                }
            }
            if !has_required {
                continue;
            }
        }

        // Language filtering could be added here
        // For now, we'll accept all samples that pass other filters

        filtered_samples.push(sample);
    }

    Ok(filtered_samples)
}

fn apply_diversity_filters(
    samples: Vec<DatasetSample>,
    diversity_filters: &crate::dataset_generator::DiversityFilters,
) -> Result<Vec<DatasetSample>> {
    // For now, implement simple diversity filtering
    // In a more sophisticated implementation, you would use embeddings for similarity
    
    if !diversity_filters.cluster_based_sampling {
        return Ok(samples);
    }

    let mut filtered_samples = Vec::new();
    let mut seen_similar: Vec<HashSet<String>> = Vec::new();

    for sample in samples {
        let normalized_input = normalize_text(&sample.input);
        let input_words: HashSet<String> = normalized_input
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();

        // Simple similarity check based on word overlap
        let mut is_similar = false;
        for seen_words in &seen_similar {
            let overlap = input_words.intersection(seen_words).count();
            let similarity = overlap as f32 / input_words.len().max(1) as f32;
            
            if similarity > diversity_filters.similarity_threshold {
                is_similar = true;
                break;
            }
        }

        if !is_similar {
            seen_similar.push(input_words);
            filtered_samples.push(sample);
        }
    }

    Ok(filtered_samples)
}

fn calculate_text_quality(text: &str) -> f32 {
    let mut score = 0.0;
    let mut factors = 0;

    // Length factor (prefer medium-length texts)
    let char_count = text.chars().count();
    if char_count >= 50 && char_count <= 2000 {
        score += 0.3;
    } else if char_count >= 20 && char_count <= 5000 {
        score += 0.1;
    }
    factors += 1;

    // Word count factor
    let word_count = text.split_whitespace().count();
    if word_count >= 10 && word_count <= 500 {
        score += 0.2;
    }
    factors += 1;

    // Sentence structure factor
    let sentence_count = text.matches(&['.', '!', '?'][..]).count();
    if sentence_count > 0 {
        let avg_words_per_sentence = word_count as f32 / sentence_count as f32;
        if avg_words_per_sentence >= 5.0 && avg_words_per_sentence <= 30.0 {
            score += 0.2;
        }
    }
    factors += 1;

    // Capitalization factor (proper sentences should start with capital letters)
    let sentences: Vec<&str> = text.split(&['.', '!', '?'][..]).collect();
    let properly_capitalized = sentences
        .iter()
        .filter(|s| !s.trim().is_empty())
        .filter(|s| s.trim().chars().next().map_or(false, |c| c.is_uppercase()))
        .count();
    
    if properly_capitalized > 0 && sentences.len() > 0 {
        let capitalization_ratio = properly_capitalized as f32 / sentences.len() as f32;
        score += capitalization_ratio * 0.1;
    }
    factors += 1;

    // Punctuation factor
    let punctuation_count = text.chars().filter(|c| c.is_ascii_punctuation()).count();
    let punctuation_ratio = punctuation_count as f32 / char_count as f32;
    if punctuation_ratio >= 0.02 && punctuation_ratio <= 0.15 {
        score += 0.1;
    }
    factors += 1;

    // Avoid texts with too many special characters or numbers
    let special_char_count = text.chars().filter(|c| !c.is_alphanumeric() && !c.is_whitespace() && !c.is_ascii_punctuation()).count();
    let special_char_ratio = special_char_count as f32 / char_count as f32;
    if special_char_ratio < 0.05 {
        score += 0.1;
    }
    factors += 1;

    // Normalize score
    if factors > 0 {
        score / factors as f32
    } else {
        0.0
    }
}

fn normalize_text(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_text_quality() {
        let good_text = "This is a well-formed sentence with proper punctuation. It has multiple sentences and good structure.";
        let score = calculate_text_quality(good_text);
        assert!(score > 0.0);

        let poor_text = "no caps no punctuation just words";
        let poor_score = calculate_text_quality(poor_text);
        assert!(score > poor_score);
    }

    #[test]
    fn test_normalize_text() {
        let text = "Hello, World! This is a TEST.";
        let normalized = normalize_text(text);
        assert_eq!(normalized, "hello world this is a test");
    }
} 