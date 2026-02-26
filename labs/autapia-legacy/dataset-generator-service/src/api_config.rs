use autapia_shared_types::DatasetMetadata;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Enhanced API endpoint definition for comprehensive dataset generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    pub name: String,
    pub description: String,
    pub method: String,
    pub endpoint: String,
    pub parameters: HashMap<String, String>,
    pub use_cases: Vec<String>,
}

/// Service definition containing multiple endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDefinition {
    pub endpoints: Vec<ApiEndpoint>,
}

/// Complete API configuration for all autapia3 services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfiguration {
    pub services: HashMap<String, ServiceDefinition>,
    pub metadata: ApiMetadata,
}

/// Metadata for the API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMetadata {
    pub version: String,
    pub total_endpoints: usize,
    pub services_count: usize,
    pub format: String,
    pub description: String,
    pub generated_at: String,
}

/// Enhanced dataset example with variations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedDatasetExample {
    pub conversations: Vec<ConversationTurn>,
    pub system: String,
    pub tools: Vec<ToolDefinition>,
    pub metadata: ExampleMetadata,
}

/// Individual conversation turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub r#type: String,
    pub function: FunctionDefinition,
}

/// Function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: ParameterSchema,
}

/// Parameter schema for function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSchema {
    pub r#type: String,
    pub properties: HashMap<String, ParameterProperty>,
    pub required: Vec<String>,
}

/// Individual parameter property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterProperty {
    pub r#type: String,
    pub description: String,
    pub items: Option<Box<ParameterProperty>>,
}

/// Tool call in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Example metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleMetadata {
    pub endpoint: String,
    pub service: String,
    pub use_case: String,
    pub variation: usize,
    pub complexity: String,
    pub scenario_type: String,
}

/// Single-turn function calling dataset example (FiftyOne format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleTurnDatasetExample {
    pub query: Vec<String>,
    pub tools: Vec<String>,
    pub answers: Vec<String>,
    pub metadata: ExampleMetadata,
}

/// Dataset format type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatasetFormat {
    SingleTurn,
    MultiTurn,
}

/// Complete enhanced dataset that supports both formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedDataset {
    pub format: DatasetFormat,
    pub multi_turn_data: Option<Vec<EnhancedDatasetExample>>,
    pub single_turn_data: Option<Vec<SingleTurnDatasetExample>>,
    pub metadata: DatasetMetadata,
}

/// Configuration for dataset generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetGenerationConfig {
    pub format: DatasetFormat,
    pub variations_per_use_case: usize,
    pub include_system_messages: bool,
    pub max_concurrent_services: usize,
    pub quality_threshold: f64,
}

impl Default for DatasetGenerationConfig {
    fn default() -> Self {
        Self {
            format: DatasetFormat::MultiTurn,
            variations_per_use_case: 3,
            include_system_messages: true,
            max_concurrent_services: 3,
            quality_threshold: 0.8,
        }
    }
}

impl Default for ApiConfiguration {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiConfiguration {
    /// Create API configuration from real autapia3 service routing schemas
    pub fn new() -> Self {
        let mut services = HashMap::new();
        
        // AI dataset creation and augmentation service for machine learning workflows
        services.insert("dataset-generator-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "GenerateDataset".to_string(),
                    description: "Generate a new synthetic dataset based on specifications".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "dataset-generator-service.GenerateDataset".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Generate synthetic training datasets for machine learning".to_string(),
                        "Augment existing datasets with additional samples".to_string(),
                        "Create labeled datasets from raw data sources".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "AugmentDataset".to_string(),
                    description: "Augment an existing dataset with additional synthetic samples".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "dataset-generator-service.AugmentDataset".to_string(),
                    parameters: [
                        ("source_dataset_id".to_string(), "ID of source dataset to augment".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Generate synthetic training datasets for machine learning".to_string(),
                        "Augment existing datasets with additional samples".to_string(),
                        "Create labeled datasets from raw data sources".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "AnalyzeDataset".to_string(),
                    description: "Analyze dataset quality, distribution, and characteristics".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "dataset-generator-service.AnalyzeDataset".to_string(),
                    parameters: [
                        ("dataset_id".to_string(), "Dataset identifier to analyze".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Generate synthetic training datasets for machine learning".to_string(),
                        "Augment existing datasets with additional samples".to_string(),
                        "Create labeled datasets from raw data sources".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "ConvertDatasetFormat".to_string(),
                    description: "Convert dataset between different formats and schemas".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "dataset-generator-service.ConvertDatasetFormat".to_string(),
                    parameters: [
                        ("source_dataset_id".to_string(), "Source dataset identifier".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Generate synthetic training datasets for machine learning".to_string(),
                        "Augment existing datasets with additional samples".to_string(),
                        "Create labeled datasets from raw data sources".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "ListDatasets".to_string(),
                    description: "List available datasets with filtering and metadata".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "dataset-generator-service.ListDatasets".to_string(),
                    parameters: [
                        ("filter_type".to_string(), "Filter by dataset type".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Generate synthetic training datasets for machine learning".to_string(),
                        "Augment existing datasets with additional samples".to_string(),
                        "Create labeled datasets from raw data sources".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "DeleteDataset".to_string(),
                    description: "Delete a dataset and its associated files".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "dataset-generator-service.DeleteDataset".to_string(),
                    parameters: [
                        ("dataset_id".to_string(), "Dataset identifier to delete".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Generate synthetic training datasets for machine learning".to_string(),
                        "Augment existing datasets with additional samples".to_string(),
                        "Create labeled datasets from raw data sources".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "GetGenerationStatus".to_string(),
                    description: "Get status of ongoing dataset generation or processing tasks".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "dataset-generator-service.GetGenerationStatus".to_string(),
                    parameters: [
                        ("task_id".to_string(), "Task identifier".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Generate synthetic training datasets for machine learning".to_string(),
                        "Augment existing datasets with additional samples".to_string(),
                        "Create labeled datasets from raw data sources".to_string(),
                    ],
                },
            ],
        });

        // Model fine-tuning service for training custom AI models on specialized datasets
        services.insert("fine-tune-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "StartTraining".to_string(),
                    description: "Start a new model fine-tuning job".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "fine-tune-service.StartTraining".to_string(),
                    parameters: [
                        ("job_name".to_string(), "Name for the training job".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
                ApiEndpoint {
                    name: "GetTrainingStatus".to_string(),
                    description: "Get detailed status and metrics for a training job".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "fine-tune-service.GetTrainingStatus".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
                ApiEndpoint {
                    name: "ListTrainingJobs".to_string(),
                    description: "List training jobs with filtering options".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "fine-tune-service.ListTrainingJobs".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
                ApiEndpoint {
                    name: "CancelTraining".to_string(),
                    description: "Cancel a running training job".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "fine-tune-service.CancelTraining".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
                ApiEndpoint {
                    name: "GetTrainingLogs".to_string(),
                    description: "Get training logs for a specific job".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "fine-tune-service.GetTrainingLogs".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
            ],
        });

        // Dynamic agent workflow orchestration and execution service
        services.insert("workflow-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "CreateWorkflow".to_string(),
                    description: "Create a new workflow definition with steps and dependencies".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "workflow-service.CreateWorkflow".to_string(),
                    parameters: [
                        ("name".to_string(), "Workflow name".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Create and manage complex multi-step workflows".to_string(),
                        "Handle parallel and sequential workflow steps".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "ExecuteWorkflow".to_string(),
                    description: "Execute a workflow with provided input data".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "workflow-service.ExecuteWorkflow".to_string(),
                    parameters: [
                        ("workflow_id".to_string(), "Workflow identifier".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Create and manage complex multi-step workflows".to_string(),
                        "Orchestrate dynamic agent-based task execution".to_string(),
                        "Handle parallel and sequential workflow steps".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "GetExecutionStatus".to_string(),
                    description: "Get the current status and progress of a workflow execution".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "workflow-service.GetExecutionStatus".to_string(),
                    parameters: [
                        ("execution_id".to_string(), "Execution identifier".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Create and manage complex multi-step workflows".to_string(),
                        "Handle parallel and sequential workflow steps".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "StopExecution".to_string(),
                    description: "Stop a running workflow execution".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "workflow-service.StopExecution".to_string(),
                    parameters: [
                        ("execution_id".to_string(), "Execution identifier".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Create and manage complex multi-step workflows".to_string(),
                        "Handle parallel and sequential workflow steps".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "ListWorkflows".to_string(),
                    description: "List available workflows with filtering and pagination".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "workflow-service.ListWorkflows".to_string(),
                    parameters: [
                        ("filter".to_string(), "Filter by workflow name or tag".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Create and manage complex multi-step workflows".to_string(),
                        "Handle parallel and sequential workflow steps".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "GetWorkflowDefinition".to_string(),
                    description: "Get detailed workflow definition and metadata".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "workflow-service.GetWorkflowDefinition".to_string(),
                    parameters: [
                        ("workflow_id".to_string(), "Workflow identifier".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Create and manage complex multi-step workflows".to_string(),
                        "Handle parallel and sequential workflow steps".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "ValidateWorkflow".to_string(),
                    description: "Validate a workflow definition for correctness".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "workflow-service.ValidateWorkflow".to_string(),
                    parameters: [
                        ("definition".to_string(), "Workflow definition to validate".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Create and manage complex multi-step workflows".to_string(),
                        "Handle parallel and sequential workflow steps".to_string(),
                    ],
                },
            ],
        });

        // Vector database service using Qdrant for storing and searching vector embeddings
        services.insert("vector-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "UpsertVector".to_string(),
                    description: "Insert or update a vector in a collection".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "vector-service.UpsertVector".to_string(),
                    parameters: [
                        ("collection_name".to_string(), "Name of the collection".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store vector embeddings".to_string(),
                        "Manage vector collections".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "SearchVector".to_string(),
                    description: "Search for similar vectors in a collection".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "vector-service.SearchVector".to_string(),
                    parameters: [
                        ("collection_name".to_string(), "Name of the collection".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store vector embeddings".to_string(),
                        "Perform similarity searches".to_string(),
                        "Manage vector collections".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "CreateCollection".to_string(),
                    description: "Create a new vector collection".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "vector-service.CreateCollection".to_string(),
                    parameters: [
                        ("collection_name".to_string(), "Name of the collection".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store vector embeddings".to_string(),
                        "Manage vector collections".to_string(),
                    ],
                },
            ],
        });

        // Document reranking service for improving search result relevance using various reranking models
        services.insert("rerank-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "Rerank".to_string(),
                    description: "Rerank a list of documents based on their relevance to a query".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "rerank-service.Rerank".to_string(),
                    parameters: [
                        ("query".to_string(), "The search query".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Rerank search results by relevance to query".to_string(),
                        "Score document relevance for ranking".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "RerankBatch".to_string(),
                    description: "Rerank multiple sets of documents for different queries".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "rerank-service.RerankBatch".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Rerank search results by relevance to query".to_string(),
                        "Score document relevance for ranking".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "GetSupportedModels".to_string(),
                    description: "Get a list of supported reranking models".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "rerank-service.GetSupportedModels".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
                ApiEndpoint {
                    name: "HealthCheck".to_string(),
                    description: "Check the health status of the reranking service".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "rerank-service.HealthCheck".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
            ],
        });

        // Retrieval-Augmented Generation service for semantic search and document retrieval
        services.insert("rag-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "search".to_string(),
                    description: "Perform semantic search over indexed documents".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "rag-service.search".to_string(),
                    parameters: [
                        ("query".to_string(), "Search query".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Perform semantic search over document collections".to_string(),
                        "Generate relevant queries for retrieval".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "generate_query".to_string(),
                    description: "Generate optimized search queries from natural language input".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "rag-service.generate_query".to_string(),
                    parameters: [
                        ("natural_query".to_string(), "Natural language query".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
                ApiEndpoint {
                    name: "index_document".to_string(),
                    description: "Index a document for semantic search".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "rag-service.index_document".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Retrieve and rank documents by relevance".to_string(),
                    ],
                },
            ],
        });

        // LLM chat & conversation service providing text completion and streaming capabilities
        services.insert("chat-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "Complete".to_string(),
                    description: "Generate a text completion for the given prompt".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "chat-service.Complete".to_string(),
                    parameters: [
                        ("prompt".to_string(), "The input prompt".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
                ApiEndpoint {
                    name: "CompleteStream".to_string(),
                    description: "Generate a streaming text completion for the given prompt".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "chat-service.CompleteStream".to_string(),
                    parameters: [
                        ("prompt".to_string(), "The input prompt".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
            ],
        });

        // Asynchronous job queue management and background task processing service
        services.insert("job-queue-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "QueueJob".to_string(),
                    description: "Queue a new job for background processing".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "job-queue-service.QueueJob".to_string(),
                    parameters: [
                        ("job_type".to_string(), "Type/category of the job".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Queue background jobs for asynchronous processing".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "GetJobStatus".to_string(),
                    description: "Get the current status and details of a job".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "job-queue-service.GetJobStatus".to_string(),
                    parameters: [
                        ("job_id".to_string(), "Job identifier".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Queue background jobs for asynchronous processing".to_string(),
                        "Track job execution status and progress".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "CancelJob".to_string(),
                    description: "Cancel a queued or running job".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "job-queue-service.CancelJob".to_string(),
                    parameters: [
                        ("job_id".to_string(), "Job identifier".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Queue background jobs for asynchronous processing".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "ListJobs".to_string(),
                    description: "List jobs with filtering and pagination".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "job-queue-service.ListJobs".to_string(),
                    parameters: [
                        ("status_filter".to_string(), "Filter by job status".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Queue background jobs for asynchronous processing".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "RetryJob".to_string(),
                    description: "Manually retry a failed job".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "job-queue-service.RetryJob".to_string(),
                    parameters: [
                        ("job_id".to_string(), "Job identifier".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Queue background jobs for asynchronous processing".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "GetQueueStats".to_string(),
                    description: "Get queue statistics and health metrics".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "job-queue-service.GetQueueStats".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Queue background jobs for asynchronous processing".to_string(),
                    ],
                },
            ],
        });

        // Intelligent chat agent service with auto-tool selection, multi-agent support, and conversation management
        services.insert("chat-agent-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "InitiateChat".to_string(),
                    description: "Start a new chat conversation with auto-tool selection capabilities".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "chat-agent-service.InitiateChat".to_string(),
                    parameters: [
                        ("agent_id".to_string(), "ID of the agent to use".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Handle conversational AI requests with auto-tool selection".to_string(),
                        "Manage multi-agent conversations and routing".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "ContinueChat".to_string(),
                    description: "Continue an existing chat conversation with context".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "chat-agent-service.ContinueChat".to_string(),
                    parameters: [
                        ("session_id".to_string(), "Session identifier".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Handle conversational AI requests with auto-tool selection".to_string(),
                        "Manage multi-agent conversations and routing".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "HandleToolPermission".to_string(),
                    description: "Handle tool permission requests and responses for auto-tool workflows".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "chat-agent-service.HandleToolPermission".to_string(),
                    parameters: [
                        ("session_id".to_string(), "Session identifier".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
                ApiEndpoint {
                    name: "GetConversationHistory".to_string(),
                    description: "Retrieve conversation history for a session".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "chat-agent-service.GetConversationHistory".to_string(),
                    parameters: [
                        ("session_id".to_string(), "Session identifier".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
                ApiEndpoint {
                    name: "ListAvailableAgents".to_string(),
                    description: "List all available agents and their capabilities".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "chat-agent-service.ListAvailableAgents".to_string(),
                    parameters: [
                        ("filter".to_string(), "Filter agents by capability or type".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
            ],
        });

        // Multi-cloud storage service for managing files across various cloud providers and local storage
        services.insert("storage-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "UploadFile".to_string(),
                    description: "Upload a file to the configured storage backend".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "storage-service.UploadFile".to_string(),
                    parameters: [
                        ("file_name".to_string(), "Name of the file".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve files from cloud storage".to_string(),
                        "Provide unified interface across storage providers".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "DownloadFile".to_string(),
                    description: "Download a file from storage".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "storage-service.DownloadFile".to_string(),
                    parameters: [
                        ("file_id".to_string(), "Unique identifier of the file to download".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve files from cloud storage".to_string(),
                        "Provide unified interface across storage providers".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "DeleteFile".to_string(),
                    description: "Delete a file from storage".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "storage-service.DeleteFile".to_string(),
                    parameters: [
                        ("file_id".to_string(), "Unique identifier of the file to delete".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve files from cloud storage".to_string(),
                        "Provide unified interface across storage providers".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "ListFiles".to_string(),
                    description: "List files in storage with optional filtering".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "storage-service.ListFiles".to_string(),
                    parameters: [
                        ("prefix".to_string(), "File name prefix filter".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve files from cloud storage".to_string(),
                        "Provide unified interface across storage providers".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "GetFileMetadata".to_string(),
                    description: "Get metadata for a specific file".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "storage-service.GetFileMetadata".to_string(),
                    parameters: [
                        ("file_id".to_string(), "Unique identifier of the file".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve files from cloud storage".to_string(),
                        "Manage file metadata and permissions".to_string(),
                        "Provide unified interface across storage providers".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "GetStorageStats".to_string(),
                    description: "Get storage usage statistics".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "storage-service.GetStorageStats".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve files from cloud storage".to_string(),
                        "Provide unified interface across storage providers".to_string(),
                    ],
                },
            ],
        });

        // System monitoring and health check service with metrics collection
        services.insert("monitor-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "GetSystemHealth".to_string(),
                    description: "Get overall system health status across all services".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "monitor-service.GetSystemHealth".to_string(),
                    parameters: [
                        ("include_details".to_string(), "Include detailed health information".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Monitor health status of all microservices".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "GetServiceMetrics".to_string(),
                    description: "Get performance metrics for specific services".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "monitor-service.GetServiceMetrics".to_string(),
                    parameters: [
                        ("service_names".to_string(), "List of service names to get metrics for".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Collect and aggregate performance metrics".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "CreateAlert".to_string(),
                    description: "Create a monitoring alert rule".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "monitor-service.CreateAlert".to_string(),
                    parameters: [
                        ("name".to_string(), "Alert rule name".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
                ApiEndpoint {
                    name: "GetActiveAlerts".to_string(),
                    description: "Get currently active alerts and their details".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "monitor-service.GetActiveAlerts".to_string(),
                    parameters: [
                        ("severity_filter".to_string(), "Filter by severity level".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Monitor health status of all microservices".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "GetServiceStatus".to_string(),
                    description: "Get detailed status information for a specific service".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "monitor-service.GetServiceStatus".to_string(),
                    parameters: [
                        ("service_name".to_string(), "Name of the service".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Monitor health status of all microservices".to_string(),
                        "Detect service failures and outages".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "StreamMetrics".to_string(),
                    description: "Stream real-time metrics data for monitoring dashboards".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "monitor-service.StreamMetrics".to_string(),
                    parameters: [
                        ("services".to_string(), "Services to stream metrics for".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Collect and aggregate performance metrics".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "TriggerHealthCheck".to_string(),
                    description: "Manually trigger health check for specific services".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "monitor-service.TriggerHealthCheck".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Monitor health status of all microservices".to_string(),
                    ],
                },
            ],
        });

        // Centralized logging service for aggregating, indexing, and querying logs from all Autapia microservices
        services.insert("logging-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "IngestLogs".to_string(),
                    description: "Ingest log entries from microservices".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "logging-service.IngestLogs".to_string(),
                    parameters: [
                        ("service_name".to_string(), "Name of the service sending logs".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Centralize logs from all microservices".to_string(),
                        "Search and filter logs across services".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "SearchLogs".to_string(),
                    description: "Search logs with advanced filtering and querying".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "logging-service.SearchLogs".to_string(),
                    parameters: [
                        ("query".to_string(), "Search query string".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Centralize logs from all microservices".to_string(),
                        "Search and filter logs across services".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "StreamLogs".to_string(),
                    description: "Stream logs in real-time with filtering".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "logging-service.StreamLogs".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Centralize logs from all microservices".to_string(),
                        "Real-time log streaming and monitoring".to_string(),
                        "Search and filter logs across services".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "GetLogMetrics".to_string(),
                    description: "Get aggregated metrics from logs".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "logging-service.GetLogMetrics".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
                ApiEndpoint {
                    name: "SetLogRetention".to_string(),
                    description: "Configure log retention policies".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "logging-service.SetLogRetention".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
            ],
        });

        // Centralized configuration management service for all Autapia microservices
        services.insert("settings-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "GetServiceConfig".to_string(),
                    description: "Retrieve configuration for a specific service".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "settings-service.GetServiceConfig".to_string(),
                    parameters: [
                        ("service_name".to_string(), "Name of the service to get configuration for".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve service configurations".to_string(),
                        "Provide centralized configuration for all microservices".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "SetServiceConfig".to_string(),
                    description: "Update configuration for a specific service".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "settings-service.SetServiceConfig".to_string(),
                    parameters: [
                        ("service_name".to_string(), "Name of the service to update configuration for".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve service configurations".to_string(),
                        "Provide centralized configuration for all microservices".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "ListServices".to_string(),
                    description: "List all services with configuration".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "settings-service.ListServices".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve service configurations".to_string(),
                        "Provide centralized configuration for all microservices".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "DeleteServiceConfig".to_string(),
                    description: "Delete configuration for a specific service".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "settings-service.DeleteServiceConfig".to_string(),
                    parameters: [
                        ("service_name".to_string(), "Name of the service to delete configuration for".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve service configurations".to_string(),
                        "Provide centralized configuration for all microservices".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "ReloadServiceConfig".to_string(),
                    description: "Trigger configuration reload for a specific service".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "settings-service.ReloadServiceConfig".to_string(),
                    parameters: [
                        ("service_name".to_string(), "Name of the service to reload configuration for".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve service configurations".to_string(),
                        "Provide centralized configuration for all microservices".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "GetEnvironments".to_string(),
                    description: "List all available environments".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "settings-service.GetEnvironments".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve service configurations".to_string(),
                        "Manage environment-specific settings".to_string(),
                        "Provide centralized configuration for all microservices".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "ValidateConfig".to_string(),
                    description: "Validate configuration against service schema".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "settings-service.ValidateConfig".to_string(),
                    parameters: [
                        ("service_name".to_string(), "Name of the service to validate configuration for".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve service configurations".to_string(),
                        "Provide centralized configuration for all microservices".to_string(),
                    ],
                },
            ],
        });

        // Intelligent task planning and execution coordination service
        services.insert("planner-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "CreatePlan".to_string(),
                    description: "Create a new execution plan for a given task".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "planner-service.CreatePlan".to_string(),
                    parameters: [
                        ("query".to_string(), "User task or query to plan for".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Multi-step task decomposition and planning".to_string(),
                        "LLM-driven intelligent planning strategies".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "GetPlan".to_string(),
                    description: "Retrieve an existing plan by ID".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "planner-service.GetPlan".to_string(),
                    parameters: [
                        ("plan_id".to_string(), "Plan identifier".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Multi-step task decomposition and planning".to_string(),
                        "LLM-driven intelligent planning strategies".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "ExecutePlan".to_string(),
                    description: "Execute a plan with step-by-step coordination".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "planner-service.ExecutePlan".to_string(),
                    parameters: [
                        ("plan_id".to_string(), "Plan identifier to execute".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Complex workflow execution coordination".to_string(),
                    ],
                },
            ],
        });

        // Application-to-Application communication service for external integrations, webhooks, and message routing
        services.insert("a2a-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "RegisterIntegration".to_string(),
                    description: "Register a new external integration or webhook endpoint".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "a2a-service.RegisterIntegration".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Handle incoming webhooks from third-party services".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "ListIntegrations".to_string(),
                    description: "List all registered integrations".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "a2a-service.ListIntegrations".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
                ApiEndpoint {
                    name: "SendMessage".to_string(),
                    description: "Send a message through registered integrations".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "a2a-service.SendMessage".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
                ApiEndpoint {
                    name: "GetIntegrationStatus".to_string(),
                    description: "Get detailed status and metrics for an integration".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "a2a-service.GetIntegrationStatus".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                    ],
                },
            ],
        });

        // Text embedding service providing vector representations for various models
        services.insert("embedding-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "Embed".to_string(),
                    description: "Generate embeddings for a single text input".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "embedding-service.Embed".to_string(),
                    parameters: [
                        ("text".to_string(), "Text to embed".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Generate text embeddings for semantic search".to_string(),
                        "Create vector representations for ML models".to_string(),
                        "Process batch embedding requests".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "EmbedBatch".to_string(),
                    description: "Generate embeddings for multiple text inputs".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "embedding-service.EmbedBatch".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Generate text embeddings for semantic search".to_string(),
                        "Create vector representations for ML models".to_string(),
                        "Process batch embedding requests".to_string(),
                    ],
                },
            ],
        });

        // Redis-based in-memory caching and chat history management service
        services.insert("in-memory-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "Set".to_string(),
                    description: "Store a key-value pair in Redis with optional expiration".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "in-memory-service.Set".to_string(),
                    parameters: [
                        ("key".to_string(), "Key to store".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve key-value pairs in Redis".to_string(),
                        "Cache frequently accessed data across services".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "Get".to_string(),
                    description: "Retrieve a value by key from Redis".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "in-memory-service.Get".to_string(),
                    parameters: [
                        ("key".to_string(), "Key to retrieve".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve key-value pairs in Redis".to_string(),
                        "Cache frequently accessed data across services".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "Delete".to_string(),
                    description: "Delete a key from Redis".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "in-memory-service.Delete".to_string(),
                    parameters: [
                        ("key".to_string(), "Key to delete".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve key-value pairs in Redis".to_string(),
                        "Cache frequently accessed data across services".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "StoreChatHistory".to_string(),
                    description: "Store chat conversation history for a session".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "in-memory-service.StoreChatHistory".to_string(),
                    parameters: [
                        ("session_id".to_string(), "Chat session identifier".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Manage chat conversation history and context".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "GetChatHistory".to_string(),
                    description: "Retrieve chat conversation history for a session".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "in-memory-service.GetChatHistory".to_string(),
                    parameters: [
                        ("session_id".to_string(), "Chat session identifier".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Manage chat conversation history and context".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "HealthCheck".to_string(),
                    description: "Check Redis connection health and service status".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "in-memory-service.HealthCheck".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Store and retrieve key-value pairs in Redis".to_string(),
                    ],
                },
            ],
        });

        // Model Context Protocol service for managing and executing tools from MCP servers
        services.insert("mcp-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "ListConfigs".to_string(),
                    description: "List all configured MCP server configurations".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "mcp-service.ListConfigs".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Connect to MCP servers via different transports".to_string(),
                        "Discover available tools from MCP servers".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "ConnectToConfig".to_string(),
                    description: "Connect to an MCP server using a specific configuration".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "mcp-service.ConnectToConfig".to_string(),
                    parameters: [
                        ("config_id".to_string(), "ID of the MCP configuration to connect to".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Connect to MCP servers via different transports".to_string(),
                        "Discover available tools from MCP servers".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "DisconnectFromConfig".to_string(),
                    description: "Disconnect from an MCP server".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "mcp-service.DisconnectFromConfig".to_string(),
                    parameters: [
                        ("config_id".to_string(), "ID of the MCP configuration to disconnect from".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Connect to MCP servers via different transports".to_string(),
                        "Discover available tools from MCP servers".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "ListTools".to_string(),
                    description: "List all available tools from connected MCP servers".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "mcp-service.ListTools".to_string(),
                    parameters: [
                        ("config_id".to_string(), "Optional config ID to filter tools from specific server".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Connect to MCP servers via different transports".to_string(),
                        "Discover available tools from MCP servers".to_string(),
                        "Execute tools with proper error handling".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "CallTool".to_string(),
                    description: "Execute a tool on an MCP server".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "mcp-service.CallTool".to_string(),
                    parameters: [
                        ("config_id".to_string(), "ID of the MCP configuration".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Connect to MCP servers via different transports".to_string(),
                        "Discover available tools from MCP servers".to_string(),
                        "Execute tools with proper error handling".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "GetStatus".to_string(),
                    description: "Get connection status for all or specific MCP configurations".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "mcp-service.GetStatus".to_string(),
                    parameters: [
                        ("config_id".to_string(), "Optional config ID to get status for specific server".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Connect to MCP servers via different transports".to_string(),
                        "Discover available tools from MCP servers".to_string(),
                    ],
                },
            ],
        });

        // Native Rust transformer service using Candle ML framework for local inference
        services.insert("candle-service".to_string(), ServiceDefinition {
            endpoints: vec![
                ApiEndpoint {
                    name: "GenerateEmbedding".to_string(),
                    description: "Generate sentence embeddings using local transformer models".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "candle-service.GenerateEmbedding".to_string(),
                    parameters: [
                        ("text".to_string(), "Text to generate embeddings for".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Generate sentence embeddings locally without API calls".to_string(),
                        "Perform text classification using local transformer models".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "ClassifyText".to_string(),
                    description: "Perform text classification using local transformer models".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "candle-service.ClassifyText".to_string(),
                    parameters: [
                        ("text".to_string(), "Text to classify".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Perform text classification using local transformer models".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "GetModelInfo".to_string(),
                    description: "Get information about loaded models".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "candle-service.GetModelInfo".to_string(),
                    parameters: [
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Perform text classification using local transformer models".to_string(),
                    ],
                },
                ApiEndpoint {
                    name: "LoadModel".to_string(),
                    description: "Load a new transformer model for inference".to_string(),
                    method: "gRPC".to_string(),
                    endpoint: "candle-service.LoadModel".to_string(),
                    parameters: [
                        ("model_name".to_string(), "HuggingFace model name or local path".to_string()),
                    ].iter().cloned().collect(),
                    use_cases: vec![
                        "Perform text classification using local transformer models".to_string(),
                    ],
                },
            ],
        });

        
        Self {
            services,
            metadata: ApiMetadata {
                version: "1.0.0".to_string(),
                total_endpoints: 92,
                services_count: 19,
                format: "autapia3_real_api".to_string(),
                description: "Real API endpoints extracted from autapia3 service routing schemas".to_string(),
                generated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            },
        }
    }
}
