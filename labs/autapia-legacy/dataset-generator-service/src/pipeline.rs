use crate::clients::ServiceClients;
use crate::dataset_generator::{DatasetConfig, RawSource, SourceType, DatasetType};
use crate::types::{
    DatasetJob, DatasetMetadata, ProcessedChunk, QAExample, DatasetSample, DatasetSplit, EmbeddingRequest, ChatRequest,
};
use crate::utils::{
    load_raw_corpus, chunk_text, filter_quality, cluster_and_sample, serialize_dataset,
};
use anyhow::{Result, anyhow};
use log::{info, warn, error};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct DatasetPipeline {
    clients: Arc<ServiceClients>,
}

impl DatasetPipeline {
    pub fn new(clients: Arc<ServiceClients>) -> Self {
        Self { clients }
    }

    pub async fn process_dataset(
        &self,
        job_id: String,
        config: DatasetConfig,
        sources: Vec<RawSource>,
        output_dir: String,
        jobs: Arc<RwLock<HashMap<String, DatasetJob>>>,
    ) -> Result<DatasetMetadata> {
        let start_time = std::time::Instant::now();
        info!("Starting dataset generation pipeline for job: {}", job_id);

        // Step 1: Load Raw Corpus
        self.update_job_progress(&jobs, &job_id, 0.1, "Loading raw corpus...").await;
        let raw_texts = self.load_raw_corpus(&sources).await?;
        info!("Loaded {} raw text documents", raw_texts.len());

        // Step 2: Chunk & Pre-Filter
        self.update_job_progress(&jobs, &job_id, 0.2, "Chunking and pre-filtering text...").await;
        let chunks = self.chunk_and_prefilter(&raw_texts, &config).await?;
        info!("Created {} text chunks", chunks.len());

        // Step 3: Embed & Retrieve (for similarity-based filtering)
        self.update_job_progress(&jobs, &job_id, 0.4, "Generating embeddings...").await;
        let embedded_chunks = self.embed_chunks(&chunks, &config).await?;
        info!("Generated embeddings for {} chunks", embedded_chunks.len());

        // Step 4: Clean & QA-Extract
        self.update_job_progress(&jobs, &job_id, 0.6, "Extracting QA pairs and cleaning data...").await;
        let qa_examples = self.extract_qa_pairs(&embedded_chunks, &config).await?;
        info!("Extracted {} QA examples", qa_examples.len());

        // Step 5: Filter & Sample
        self.update_job_progress(&jobs, &job_id, 0.8, "Filtering and sampling dataset...").await;
        let filtered_samples = self.filter_and_sample(&qa_examples, &config).await?;
        info!("Filtered to {} final samples", filtered_samples.len());

        // Step 6: Serialize JSONL & Publish Dataset
        self.update_job_progress(&jobs, &job_id, 0.9, "Serializing and publishing dataset...").await;
        let dataset_metadata = self.serialize_and_publish(&filtered_samples, &config, &output_dir, &job_id).await?;

        let processing_time = start_time.elapsed().as_secs_f64();
        info!("Dataset generation completed in {:.2} seconds", processing_time);

        self.update_job_progress(&jobs, &job_id, 1.0, "Dataset generation completed successfully").await;

        Ok(dataset_metadata)
    }

    async fn load_raw_corpus(&self, sources: &[RawSource]) -> Result<Vec<(String, String, HashMap<String, String>)>> {
        let mut raw_texts = Vec::new();

        for source in sources {
            info!("Processing data source: {} (type: {:?})", source.path, source.source_type());
            
            match source.source_type() {
                SourceType::PlainText => {
                    match load_raw_corpus::load_plain_text_files(&source.path).await {
                        Ok(texts) => {
                            info!("Loaded {} plain text files from: {}", texts.len(), source.path);
                            for (filename, content) in texts {
                                let mut metadata = source.metadata.clone();
                                metadata.insert("source_file".to_string(), filename.clone());
                                metadata.insert("source_type".to_string(), "plain_text".to_string());
                                raw_texts.push((filename, content, metadata));
                            }
                        }
                        Err(e) => {
                            error!("Failed to load plain text files from '{}': {}", source.path, e);
                            return Err(anyhow!("Failed to load plain text files from '{}': {}", source.path, e));
                        }
                    }
                }
                SourceType::Jsonl => {
                    match load_raw_corpus::load_jsonl_files(&source.path).await {
                        Ok(texts) => {
                            info!("Loaded {} JSONL entries from: {}", texts.len(), source.path);
                            for (filename, content) in texts {
                                let mut metadata = source.metadata.clone();
                                metadata.insert("source_file".to_string(), filename.clone());
                                metadata.insert("source_type".to_string(), "jsonl".to_string());
                                raw_texts.push((filename, content, metadata));
                            }
                        }
                        Err(e) => {
                            error!("Failed to load JSONL files from '{}': {}", source.path, e);
                            return Err(anyhow!("Failed to load JSONL files from '{}': {}", source.path, e));
                        }
                    }
                }
                SourceType::CsvTsv => {
                    match load_raw_corpus::load_csv_tsv_files(&source.path).await {
                        Ok(texts) => {
                            info!("Loaded {} CSV/TSV entries from: {}", texts.len(), source.path);
                            for (filename, content) in texts {
                                let mut metadata = source.metadata.clone();
                                metadata.insert("source_file".to_string(), filename.clone());
                                metadata.insert("source_type".to_string(), "csv_tsv".to_string());
                                raw_texts.push((filename, content, metadata));
                            }
                        }
                        Err(e) => {
                            error!("Failed to load CSV/TSV files from '{}': {}", source.path, e);
                            return Err(anyhow!("Failed to load CSV/TSV files from '{}': {}", source.path, e));
                        }
                    }
                }
                SourceType::ApiSpecification => {
                    // For API specifications, we treat the path as either a file path to a JSON file
                    // or the raw JSON data itself (for dynamic/generated API specs)
                    info!("Processing API specification data for function calling dataset");
                    
                    let api_spec_content = if source.path.starts_with('{') || source.path.starts_with('[') {
                        // Treat as raw JSON data
                        source.path.clone()
                    } else {
                        // Treat as file path
                        match tokio::fs::read_to_string(&source.path).await {
                            Ok(content) => content,
                            Err(e) => {
                                error!("Failed to read API specification file '{}': {}", source.path, e);
                                return Err(anyhow!("Failed to read API specification file '{}': {}", source.path, e));
                            }
                        }
                    };

                    // Parse and validate JSON
                    match serde_json::from_str::<serde_json::Value>(&api_spec_content) {
                        Ok(_) => {
                            let mut metadata = source.metadata.clone();
                            metadata.insert("source_file".to_string(), "api_specification.json".to_string());
                            metadata.insert("source_type".to_string(), "api_specification".to_string());
                            metadata.insert("format".to_string(), "openapi_json".to_string());
                            
                            raw_texts.push((
                                "api_specification.json".to_string(),
                                api_spec_content,
                                metadata
                            ));
                            info!("Successfully loaded API specification data");
                        }
                        Err(e) => {
                            error!("Invalid JSON in API specification: {}", e);
                            return Err(anyhow!("Invalid JSON in API specification: {}", e));
                        }
                    }
                }
            }
        }

        if raw_texts.is_empty() {
            let error_msg = format!("No text content found in provided sources: {:?}", 
                sources.iter().map(|s| &s.path).collect::<Vec<_>>());
            error!("{}", error_msg);
            return Err(anyhow!(error_msg));
        }

        info!("Successfully loaded {} text documents from {} sources", raw_texts.len(), sources.len());
        Ok(raw_texts)
    }

    async fn chunk_and_prefilter(
        &self,
        raw_texts: &[(String, String, HashMap<String, String>)],
        config: &DatasetConfig,
    ) -> Result<Vec<ProcessedChunk>> {
        let mut chunks = Vec::new();

        let chunking_config = config.processing.as_ref()
            .and_then(|p| p.chunking.as_ref())
            .ok_or_else(|| anyhow!("Chunking configuration is required"))?;

        for (filename, content, metadata) in raw_texts {
            let text_chunks = chunk_text::chunk_text(
                content,
                chunking_config.chunk_size as usize,
                chunking_config.chunk_overlap as usize,
                &chunking_config.strategy,
            )?;

            for (i, chunk_text) in text_chunks.into_iter().enumerate() {
                let chunk = ProcessedChunk {
                    id: format!("{}_{}", filename, i),
                    text: chunk_text,
                    embedding: None,
                    metadata: metadata.clone(),
                    quality_score: 0.0, // Will be calculated later
                    source_file: filename.clone(),
                };
                chunks.push(chunk);
            }
        }

        // Apply basic quality filters
        if let Some(filtering_config) = &config.filtering {
            if let Some(quality_filters) = &filtering_config.quality {
                chunks = filter_quality::apply_basic_filters(chunks, quality_filters)?;
            }
        }

        Ok(chunks)
    }

    async fn embed_chunks(
        &self,
        chunks: &[ProcessedChunk],
        config: &DatasetConfig,
    ) -> Result<Vec<ProcessedChunk>> {
        let embedding_config = config.processing.as_ref()
            .and_then(|p| p.embedding.as_ref())
            .ok_or_else(|| anyhow!("Embedding configuration is required"))?;

        let mut embedded_chunks = Vec::new();
        let batch_size = embedding_config.batch_size as usize;

        for chunk_batch in chunks.chunks(batch_size) {
            let mut batch_results = Vec::new();

            for chunk in chunk_batch {
                let embedding_request = crate::types_minimal::EmbeddingRequest {
                    text: chunk.text.clone(),
                    model: embedding_config.model.clone(),
                    provider: embedding_config.provider.clone(),
                };

                match self.clients.generate_embedding(embedding_request).await {
                    Ok(response) => {
                        let mut embedded_chunk = chunk.clone();
                        embedded_chunk.embedding = Some(response.embedding);
                        batch_results.push(embedded_chunk);
                    }
                    Err(e) => {
                        warn!("Failed to generate embedding for chunk {}: {}", chunk.id, e);
                        // Include chunk without embedding
                        batch_results.push(chunk.clone());
                    }
                }
            }

            embedded_chunks.extend(batch_results);
        }

        Ok(embedded_chunks)
    }

    async fn extract_qa_pairs(
        &self,
        chunks: &[ProcessedChunk],
        config: &DatasetConfig,
    ) -> Result<Vec<QAExample>> {
        let qa_config = config.processing.as_ref()
            .and_then(|p| p.qa_extraction.as_ref())
            .ok_or_else(|| anyhow!("QA extraction configuration is required"))?;

        let mut qa_examples = Vec::new();

        for chunk in chunks {
            // Generate questions and answers based on dataset type
            let prompt = self.create_qa_extraction_prompt(&config.dataset_type(), &chunk.text, qa_config.questions_per_chunk)?;

            let chat_request = crate::types_minimal::ChatRequest {
                prompt,
                model: qa_config.model.clone(),
                provider: qa_config.provider.clone(),
                temperature: qa_config.temperature,
                max_tokens: 2000,
            };

            match self.clients.generate_chat_completion(chat_request).await {
                Ok(response) => {
                    let parsed_qa = self.parse_qa_response(&response.response, &chunk.text, &chunk.metadata)?;
                    qa_examples.extend(parsed_qa);
                }
                Err(e) => {
                    warn!("Failed to extract QA pairs from chunk {}: {}", chunk.id, e);
                }
            }
        }

        Ok(qa_examples)
    }

    async fn filter_and_sample(
        &self,
        qa_examples: &[QAExample],
        config: &DatasetConfig,
    ) -> Result<Vec<DatasetSample>> {
        let mut samples: Vec<DatasetSample> = qa_examples
            .iter()
            .enumerate()
            .map(|(i, qa)| DatasetSample {
                id: format!("sample_{}", i),
                input: qa.question.clone(),
                output: qa.answer.clone(),
                instruction: None,
                context: Some(qa.context.clone()),
                metadata: qa.metadata.clone(),
                quality_score: qa.quality_score,
                split: DatasetSplit::Train, // Will be assigned later
            })
            .collect();

        // Apply quality filtering
        if let Some(filtering_config) = &config.filtering {
            samples = filter_quality::apply_quality_filters(samples, filtering_config)?;
        }

        // Apply sampling strategy
        if let Some(sampling_config) = &config.sampling {
            samples = cluster_and_sample::apply_sampling_strategy(samples, sampling_config)?;
        }

        // Assign train/validation/test splits
        self.assign_splits(&mut samples, config)?;

        Ok(samples)
    }

    async fn serialize_and_publish(
        &self,
        samples: &[DatasetSample],
        config: &DatasetConfig,
        output_dir: &str,
        job_id: &str,
    ) -> Result<DatasetMetadata> {
        let dataset_id = Uuid::new_v4().to_string();
        // Use job_id as filename with .jsonl extension as requested
        let filename = format!("{}.jsonl", job_id);
        let output_path = Path::new(output_dir).join(&filename);

        // Serialize the dataset
        let file_size = serialize_dataset::serialize_to_jsonl(samples, &output_path).await?;

        // Create metadata
        let train_samples = samples.iter().filter(|s| matches!(s.split, DatasetSplit::Train)).count() as u64;
        let validation_samples = samples.iter().filter(|s| matches!(s.split, DatasetSplit::Validation)).count() as u64;
        let test_samples = samples.iter().filter(|s| matches!(s.split, DatasetSplit::Test)).count() as u64;

        let metadata = DatasetMetadata {
            dataset_id,
            name: config.name.clone(),
            description: config.description.clone(),
            total_samples: samples.len() as u64,
            train_samples,
            validation_samples,
            test_samples,
            created_at: chrono::Utc::now().to_rfc3339(),
            output_path: output_path.to_string_lossy().to_string(),
            file_size_bytes: file_size,
        };

        info!("Dataset published to: {}", output_path.display());
        Ok(metadata)
    }

    fn create_qa_extraction_prompt(
        &self,
        dataset_type: &DatasetType,
        context: &str,
        questions_per_chunk: u32,
    ) -> Result<String> {
        let prompt = match dataset_type {
            DatasetType::QuestionAnswer => {
                format!(
                    "Based on the following text, generate {} high-quality question-answer pairs. \
                    Each question should be answerable from the given context. \
                    Format your response as JSON with 'questions' array containing objects with 'question' and 'answer' fields.\n\n\
                    Context: {}\n\n\
                    Generate {} question-answer pairs:",
                    questions_per_chunk, context, questions_per_chunk
                )
            }
            DatasetType::InstructionFollowing => {
                format!(
                    "Based on the following text, generate {} instruction-following examples. \
                    Each example should have a clear instruction and appropriate response. \
                    Format your response as JSON with 'instructions' array containing objects with 'instruction' and 'response' fields.\n\n\
                    Context: {}\n\n\
                    Generate {} instruction-response pairs:",
                    questions_per_chunk, context, questions_per_chunk
                )
            }
            DatasetType::Summarization => {
                format!(
                    "Based on the following text, generate {} summarization examples. \
                    Create different types of summaries (brief, detailed, key points). \
                    Format your response as JSON with 'summaries' array containing objects with 'instruction' and 'summary' fields.\n\n\
                    Text to summarize: {}\n\n\
                    Generate {} summarization examples:",
                    questions_per_chunk, context, questions_per_chunk
                )
            }
            DatasetType::FunctionCalling => {
                format!(
                    "Based on the following API documentation, generate {} function calling examples. \
                    Create diverse examples showing how to call the API functions with different parameters. \
                    Include both successful calls and edge cases. \
                    Format your response as JSON with 'function_calls' array containing objects with 'user_query', 'function_name', 'arguments', and 'expected_response' fields.\n\n\
                    API Documentation: {}\n\n\
                    Generate {} function calling examples:",
                    questions_per_chunk, context, questions_per_chunk
                )
            }
            _ => {
                format!(
                    "Based on the following text, generate {} training examples relevant to the content. \
                    Format your response as JSON with 'examples' array containing objects with 'input' and 'output' fields.\n\n\
                    Context: {}\n\n\
                    Generate {} examples:",
                    questions_per_chunk, context, questions_per_chunk
                )
            }
        };

        Ok(prompt)
    }

    fn parse_qa_response(
        &self,
        response: &str,
        context: &str,
        metadata: &HashMap<String, String>,
    ) -> Result<Vec<QAExample>> {
        // Try to parse JSON response
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(response) {
            let mut qa_examples = Vec::new();

            // Handle different JSON structures
            if let Some(questions) = json_value.get("questions").and_then(|v| v.as_array()) {
                for q in questions {
                    if let (Some(question), Some(answer)) = (
                        q.get("question").and_then(|v| v.as_str()),
                        q.get("answer").and_then(|v| v.as_str()),
                    ) {
                        qa_examples.push(QAExample {
                            question: question.to_string(),
                            answer: answer.to_string(),
                            context: context.to_string(),
                            quality_score: 0.8, // Default score, could be improved with quality assessment
                            metadata: metadata.clone(),
                        });
                    }
                }
            }

            if !qa_examples.is_empty() {
                return Ok(qa_examples);
            }
        }

        // Fallback: simple text parsing
        warn!("Failed to parse JSON response, using fallback text parsing");
        Ok(vec![QAExample {
            question: "What is the main topic of this text?".to_string(),
            answer: response.chars().take(200).collect::<String>() + "...",
            context: context.to_string(),
            quality_score: 0.5, // Lower score for fallback
            metadata: metadata.clone(),
        }])
    }

    fn assign_splits(&self, samples: &mut [DatasetSample], config: &DatasetConfig) -> Result<()> {
        if let Some(sampling_config) = &config.sampling {
            let total = samples.len();
            let train_count = (total as f32 * sampling_config.train_split) as usize;
            let val_count = (total as f32 * sampling_config.validation_split) as usize;

            // Shuffle samples for random split assignment
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            samples.shuffle(&mut rng);

            for (i, sample) in samples.iter_mut().enumerate() {
                sample.split = if i < train_count {
                    DatasetSplit::Train
                } else if i < train_count + val_count {
                    DatasetSplit::Validation
                } else {
                    DatasetSplit::Test
                };
            }
        }

        Ok(())
    }

    async fn update_job_progress(
        &self,
        jobs: &Arc<RwLock<HashMap<String, DatasetJob>>>,
        job_id: &str,
        progress: f32,
        message: &str,
    ) {
        let mut jobs_map = jobs.write().await;
        if let Some(job) = jobs_map.get_mut(job_id) {
            job.progress = progress;
            job.message = message.to_string();
            job.updated_at = chrono::Utc::now();
        }
    }
} 