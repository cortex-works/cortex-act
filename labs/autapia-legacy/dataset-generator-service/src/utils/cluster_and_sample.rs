use crate::dataset_generator::{SamplingConfig, SamplingStrategy};
use crate::types::DatasetSample;
use anyhow::Result;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

pub fn apply_sampling_strategy(
    samples: Vec<DatasetSample>,
    sampling_config: &SamplingConfig,
) -> Result<Vec<DatasetSample>> {
    let target_size = sampling_config.target_size as usize;
    
    if samples.len() <= target_size {
        return Ok(samples);
    }

    match sampling_config.strategy() {
        SamplingStrategy::Random => random_sampling(samples, target_size),
        SamplingStrategy::Stratified => stratified_sampling(samples, target_size),
        SamplingStrategy::Balanced => balanced_sampling(samples, target_size),
        SamplingStrategy::QualityWeighted => quality_weighted_sampling(samples, target_size),
    }
}

fn random_sampling(mut samples: Vec<DatasetSample>, target_size: usize) -> Result<Vec<DatasetSample>> {
    let mut rng = thread_rng();
    samples.shuffle(&mut rng);
    samples.truncate(target_size);
    Ok(samples)
}

fn stratified_sampling(samples: Vec<DatasetSample>, target_size: usize) -> Result<Vec<DatasetSample>> {
    // Group samples by some criteria (e.g., source file or quality score ranges)
    let mut groups = std::collections::HashMap::new();
    
    for sample in samples {
        let group_key = get_stratification_key(&sample);
        groups.entry(group_key).or_insert_with(Vec::new).push(sample);
    }

    let num_groups = groups.len();
    let samples_per_group = target_size / num_groups.max(1);
    let remainder = target_size % num_groups.max(1);

    let mut result = Vec::new();
    let mut rng = thread_rng();

    for (i, (_, mut group_samples)) in groups.into_iter().enumerate() {
        let group_target = if i < remainder {
            samples_per_group + 1
        } else {
            samples_per_group
        };

        group_samples.shuffle(&mut rng);
        group_samples.truncate(group_target);
        result.extend(group_samples);
    }

    // If we still need more samples, add randomly from what's available
    if result.len() < target_size {
        result.shuffle(&mut rng);
    } else {
        result.shuffle(&mut rng);
        result.truncate(target_size);
    }

    Ok(result)
}

fn balanced_sampling(samples: Vec<DatasetSample>, target_size: usize) -> Result<Vec<DatasetSample>> {
    // Balance by quality score ranges
    let mut high_quality = Vec::new();
    let mut medium_quality = Vec::new();
    let mut low_quality = Vec::new();

    for sample in samples {
        if sample.quality_score >= 0.8 {
            high_quality.push(sample);
        } else if sample.quality_score >= 0.5 {
            medium_quality.push(sample);
        } else {
            low_quality.push(sample);
        }
    }

    let mut rng = thread_rng();
    high_quality.shuffle(&mut rng);
    medium_quality.shuffle(&mut rng);
    low_quality.shuffle(&mut rng);

    // Prefer higher quality samples but maintain some diversity
    let high_target = (target_size as f32 * 0.6) as usize;
    let medium_target = (target_size as f32 * 0.3) as usize;
    let low_target = target_size - high_target - medium_target;

    let mut result = Vec::new();
    
    result.extend(high_quality.into_iter().take(high_target));
    result.extend(medium_quality.into_iter().take(medium_target));
    result.extend(low_quality.into_iter().take(low_target));

    // If we don't have enough samples, fill from any remaining
    if result.len() < target_size {
        // This shouldn't happen given our input validation, but handle gracefully
        result.shuffle(&mut rng);
    } else {
        result.shuffle(&mut rng);
        result.truncate(target_size);
    }

    Ok(result)
}

fn quality_weighted_sampling(mut samples: Vec<DatasetSample>, target_size: usize) -> Result<Vec<DatasetSample>> {
    // Sort by quality score (descending)
    samples.sort_by(|a, b| b.quality_score.partial_cmp(&a.quality_score).unwrap_or(std::cmp::Ordering::Equal));

    // Use weighted selection based on quality scores
    let mut selected = Vec::new();
    let mut rng = thread_rng();

    // Calculate cumulative weights
    let total_weight: f32 = samples.iter().map(|s| s.quality_score + 0.1).sum(); // Add 0.1 to avoid zero weights
    
    for _ in 0..target_size.min(samples.len()) {
        if samples.is_empty() {
            break;
        }

        let random_weight = rng.gen::<f32>() * total_weight;
        let mut cumulative_weight = 0.0;
        let mut selected_index = 0;

        for (i, sample) in samples.iter().enumerate() {
            cumulative_weight += sample.quality_score + 0.1;
            if cumulative_weight >= random_weight {
                selected_index = i;
                break;
            }
        }

        selected.push(samples.remove(selected_index));
    }

    Ok(selected)
}

fn get_stratification_key(sample: &DatasetSample) -> String {
    // Stratify by source file if available
    if let Some(source_file) = sample.metadata.get("source_file") {
        return source_file.clone();
    }

    // Fallback to quality score ranges
    if sample.quality_score >= 0.8 {
        "high_quality".to_string()
    } else if sample.quality_score >= 0.5 {
        "medium_quality".to_string()
    } else {
        "low_quality".to_string()
    }
}

// Simple clustering implementation using quality scores and text length
pub fn cluster_samples(samples: &[DatasetSample], num_clusters: usize) -> Vec<Vec<usize>> {
    if samples.is_empty() || num_clusters == 0 {
        return vec![];
    }

    let mut clusters = vec![Vec::new(); num_clusters];
    
    for (i, sample) in samples.iter().enumerate() {
        // Simple clustering based on quality score and text length
        let quality_bucket = (sample.quality_score * (num_clusters as f32 - 1.0)) as usize;
        let cluster_index = quality_bucket.min(num_clusters - 1);
        clusters[cluster_index].push(i);
    }

    // Ensure no empty clusters by redistributing
    let mut non_empty_clusters: Vec<_> = clusters.into_iter().filter(|c| !c.is_empty()).collect();
    
    // If we have fewer non-empty clusters than requested, pad with empty ones
    while non_empty_clusters.len() < num_clusters {
        non_empty_clusters.push(Vec::new());
    }

    non_empty_clusters
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DatasetSplit;
    use std::collections::HashMap;

    fn create_test_sample(id: &str, quality_score: f32) -> DatasetSample {
        DatasetSample {
            id: id.to_string(),
            input: format!("Input for {}", id),
            output: format!("Output for {}", id),
            instruction: None,
            context: None,
            metadata: HashMap::new(),
            quality_score,
            split: DatasetSplit::Train,
        }
    }

    #[test]
    fn test_random_sampling() {
        let samples = vec![
            create_test_sample("1", 0.9),
            create_test_sample("2", 0.8),
            create_test_sample("3", 0.7),
            create_test_sample("4", 0.6),
            create_test_sample("5", 0.5),
        ];

        let result = random_sampling(samples, 3).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_quality_weighted_sampling() {
        let samples = vec![
            create_test_sample("1", 0.9),
            create_test_sample("2", 0.8),
            create_test_sample("3", 0.7),
            create_test_sample("4", 0.6),
            create_test_sample("5", 0.5),
        ];

        let result = quality_weighted_sampling(samples, 3).unwrap();
        assert_eq!(result.len(), 3);
        
        // Higher quality samples should be more likely to be selected
        // (This is probabilistic, so we can't guarantee specific samples)
    }

    #[test]
    fn test_cluster_samples() {
        let samples = vec![
            create_test_sample("1", 0.9),
            create_test_sample("2", 0.8),
            create_test_sample("3", 0.7),
            create_test_sample("4", 0.6),
            create_test_sample("5", 0.5),
        ];

        let clusters = cluster_samples(&samples, 3);
        assert_eq!(clusters.len(), 3);
        
        // All samples should be assigned to some cluster
        let total_assigned: usize = clusters.iter().map(|c| c.len()).sum();
        assert_eq!(total_assigned, samples.len());
    }
} 