use crate::error::{FineTuneError, Result};
use log::{debug, error};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct EmbeddingServiceClient {
    client: Client,
    base_url: String,
}

#[derive(Serialize)]
struct EmbeddingRequest {
    text: String,
    model: Option<String>,
}

#[derive(Serialize)]
struct BatchEmbeddingRequest {
    texts: Vec<String>,
    model: Option<String>,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}

impl EmbeddingServiceClient {
    pub async fn new(embedding_service_url: &str) -> Result<Self> {
        debug!("Connecting to embedding service at: {}", embedding_service_url);
        
        Ok(Self {
            client: Client::new(),
            base_url: embedding_service_url.to_string(),
        })
    }

    pub async fn generate_embedding(&mut self, text: &str) -> Result<Vec<f32>> {
        debug!("Generating embedding for text length: {}", text.len());

        let request = EmbeddingRequest {
            text: text.to_string(),
            model: None,
        };

        let response = self.client
            .post(&format!("{}/embed", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to generate embedding: {}", e);
                FineTuneError::EmbeddingService(e.to_string())
            })?;

        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse embedding response: {}", e);
                FineTuneError::EmbeddingService(e.to_string())
            })?;

        debug!("Generated embedding with {} dimensions", embedding_response.embedding.len());
        Ok(embedding_response.embedding)
    }

    pub async fn generate_batch_embeddings(&mut self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        debug!("Generating batch embeddings for {} texts", texts.len());

        let request = BatchEmbeddingRequest {
            texts,
            model: None, // Use default model
        };

        let response = self.client
            .post(&format!("{}/batch_embed", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to generate batch embeddings: {}", e);
                FineTuneError::EmbeddingService(e.to_string())
            })?;

        let embeddings: Vec<Vec<f32>> = response
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse batch embedding response: {}", e);
                FineTuneError::EmbeddingService(e.to_string())
            })?;

        debug!("Generated {} embeddings", embeddings.len());
        Ok(embeddings)
    }

    /// Generate embeddings for training data chunks
    pub async fn embed_training_chunks(
        &mut self,
        chunks: &[String],
        batch_size: usize,
    ) -> Result<Vec<Vec<f32>>> {
        debug!("Embedding {} training chunks with batch size {}", chunks.len(), batch_size);

        let mut all_embeddings = Vec::new();

        // Process in batches to avoid overwhelming the service
        for chunk_batch in chunks.chunks(batch_size) {
            let batch_texts = chunk_batch.iter().cloned().collect();
            let batch_embeddings = self.generate_batch_embeddings(batch_texts).await?;
            all_embeddings.extend(batch_embeddings);
        }

        debug!("Successfully embedded all {} chunks", chunks.len());
        Ok(all_embeddings)
    }

    /// Generate embedding for a query with specific model
    pub async fn generate_embedding_with_model(
        &mut self,
        text: &str,
        model: &str,
    ) -> Result<Vec<f32>> {
        debug!("Generating embedding for text with model: {}", model);

        let request = EmbeddingRequest {
            text: text.to_string(),
            model: Some(model.to_string()),
        };

        let response = self.client
            .post(&format!("{}/embed", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to generate embedding with model {}: {}", model, e);
                FineTuneError::EmbeddingService(e.to_string())
            })?;

        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse embedding response: {}", e);
                FineTuneError::EmbeddingService(e.to_string())
            })?;

        debug!("Generated embedding with {} dimensions using model {}", embedding_response.embedding.len(), model);
        Ok(embedding_response.embedding)
    }

    /// Calculate similarity between two texts using embeddings
    pub async fn calculate_similarity(&mut self, text1: &str, text2: &str) -> Result<f32> {
        debug!("Calculating similarity between two texts");

        let texts = vec![text1.to_string(), text2.to_string()];
        let embeddings = self.generate_batch_embeddings(texts).await?;

        if embeddings.len() != 2 {
            return Err(FineTuneError::EmbeddingService(
                "Expected 2 embeddings for similarity calculation".to_string(),
            ));
        }

        let similarity = cosine_similarity(&embeddings[0], &embeddings[1]);
        debug!("Calculated similarity: {}", similarity);

        Ok(similarity)
    }

    /// Find the most similar text from a list
    pub async fn find_most_similar(
        &mut self,
        query_text: &str,
        candidate_texts: &[String],
    ) -> Result<(usize, f32)> {
        debug!("Finding most similar text from {} candidates", candidate_texts.len());

        let query_embedding = self.generate_embedding(query_text).await?;
        let candidate_embeddings = self.generate_batch_embeddings(candidate_texts.to_vec()).await?;

        let mut best_index = 0;
        let mut best_similarity = -1.0;

        for (i, candidate_embedding) in candidate_embeddings.iter().enumerate() {
            let similarity = cosine_similarity(&query_embedding, candidate_embedding);
            if similarity > best_similarity {
                best_similarity = similarity;
                best_index = i;
            }
        }

        debug!("Most similar text at index {} with similarity {}", best_index, best_similarity);
        Ok((best_index, best_similarity))
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
} 