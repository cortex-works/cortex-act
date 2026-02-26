# Dataset Generator Service API Documentation

## Service Overview

The Dataset Generator Service is a microservice designed for creating high-quality training datasets for machine learning workflows. It specializes in generating comprehensive API documentation datasets for function-calling scenarios, covering 85+ API endpoints across the Autapia platform.

### Service Details
- **Service Name**: `dataset-generator-service`
- **Version**: `0.1.0`
- **gRPC Port**: `20090`
- **Admin HTTP Port**: `20091`
- **Database**: PostgreSQL (database-per-service pattern)

### Key Capabilities
- Generate synthetic training datasets for machine learning
- Create enhanced API documentation datasets with multiple variations
- Augment existing datasets with additional samples
- Convert between different dataset formats (CSV, JSON, Parquet)
- Analyze dataset quality and distribution
- Support for multiple concurrent dataset generation jobs

## gRPC API

### Service Definition
```protobuf
syntax = "proto3";
package dataset_generator;

service DatasetGenerator {
  rpc GenerateDataset(GenerateDatasetRequest) returns (GenerateDatasetResponse);
  rpc GenerateEnhancedApiDataset(GenerateEnhancedApiDatasetRequest) returns (GenerateEnhancedApiDatasetResponse);
  rpc GetGenerationStatus(GetGenerationStatusRequest) returns (GetGenerationStatusResponse);
  rpc ListDatasets(ListDatasetsRequest) returns (ListDatasetsResponse);
  rpc DeleteDataset(DeleteDatasetRequest) returns (DeleteDatasetResponse);
}
```

### Methods

#### GenerateDataset
Generates a dataset from raw corpus data sources.

**Request:**
```protobuf
message GenerateDatasetRequest {
  string job_id = 1;                    // Optional job identifier
  DatasetConfig config = 2;             // Dataset configuration
  repeated RawSource sources = 3;       // Data sources
}
```

**Response:**
```protobuf
message GenerateDatasetResponse {
  string job_id = 1;                    // Generated job ID
  GenerationStatus status = 2;          // Current status
  string message = 3;                   // Status message
  optional DatasetInfo dataset_info = 4; // Dataset information
}
```

**Usage Example:**
```bash
grpcurl -plaintext \
  -d '{
    "config": {
      "name": "my_dataset", 
      "dataset_type": "FUNCTION_CALLING",
      "processing": {"chunking": {"chunk_size": 512}},
      "output": {"format": "OUTPUT_JSONL"}
    },
    "sources": [{
      "source_type": "API_SPECIFICATION",
      "path": "/path/to/openapi.json"
    }]
  }' \
  localhost:20090 \
  dataset_generator.DatasetGenerator/GenerateDataset
```

#### GenerateEnhancedApiDataset
Generates comprehensive API documentation datasets with enhanced variations.

**Request:**
```protobuf
message GenerateEnhancedApiDatasetRequest {
  int32 variations_per_use_case = 1;    // Number of variations per use case
}
```

**Response:**
```protobuf
message GenerateEnhancedApiDatasetResponse {
  string job_id = 1;                    // Job identifier
  GenerationStatus status = 2;          // Current status
  string message = 3;                   // Status message
  int32 estimated_examples = 4;         // Estimated total examples
}
```

**Usage Example:**
```bash
grpcurl -plaintext \
  -d '{"variations_per_use_case": 3}' \
  localhost:20090 \
  dataset_generator.DatasetGenerator/GenerateEnhancedApiDataset
```

#### GetGenerationStatus
Retrieves the status of a dataset generation job.

**Request:**
```protobuf
message GetGenerationStatusRequest {
  string job_id = 1;                    // Job identifier
}
```

**Response:**
```protobuf
message GetGenerationStatusResponse {
  string job_id = 1;                    // Job identifier
  GenerationStatus status = 2;          // Current status
  string message = 3;                   // Status message
  float progress = 4;                   // Progress (0.0 to 1.0)
  optional DatasetInfo dataset_info = 5; // Dataset information
}
```

#### ListDatasets
Lists available datasets with pagination and filtering.

**Request:**
```protobuf
message ListDatasetsRequest {
  uint32 page = 1;                      // Page number (1-based)
  uint32 page_size = 2;                 // Results per page
}
```

**Response:**
```protobuf
message ListDatasetsResponse {
  repeated DatasetInfo datasets = 1;    // Dataset list
  uint32 total_count = 2;               // Total datasets
  uint32 page = 3;                      // Current page
  uint32 page_size = 4;                 // Page size
}
```

#### DeleteDataset
Deletes a generated dataset.

**Request:**
```protobuf
message DeleteDatasetRequest {
  string dataset_id = 1;                // Dataset identifier
}
```

**Response:**
```protobuf
message DeleteDatasetResponse {
  bool success = 1;                     // Success flag
  string message = 2;                   // Result message
}
```

### Data Types

#### DatasetConfig
```protobuf
message DatasetConfig {
  string name = 1;                      // Dataset name
  string description = 2;               // Dataset description
  DatasetType dataset_type = 3;         // Type of dataset
  ProcessingConfig processing = 4;      // Processing options
  FilteringConfig filtering = 5;        // Filtering options
  SamplingConfig sampling = 6;          // Sampling options
  OutputConfig output = 7;              // Output configuration
}
```

#### DatasetType
```protobuf
enum DatasetType {
  QUESTION_ANSWER = 0;
  INSTRUCTION_FOLLOWING = 1;
  CLASSIFICATION = 2;
  SUMMARIZATION = 3;
  CONVERSATION = 4;
  FUNCTION_CALLING = 5;                 // API documentation for function calling
}
```

#### GenerationStatus
```protobuf
enum GenerationStatus {
  PENDING = 0;                          // Job submitted but not started
  PROCESSING = 1;                       // Job in progress
  COMPLETED = 2;                        // Job completed successfully
  FAILED = 3;                           // Job failed with error
}
```

#### DatasetInfo
```protobuf
message DatasetInfo {
  string dataset_id = 1;                // Unique dataset ID
  string name = 2;                      // Dataset name
  string description = 3;               // Dataset description
  uint64 total_samples = 4;             // Total number of samples
  uint64 train_samples = 5;             // Training samples
  uint64 validation_samples = 6;        // Validation samples
  uint64 test_samples = 7;              // Test samples
  string created_at = 8;                // Creation timestamp
  string output_path = 9;               // File system path
  uint64 file_size_bytes = 10;          // File size in bytes
}
```

## HTTP Admin API

The service exposes administrative endpoints for health monitoring and configuration management.

### Base URL
```
http://localhost:20091
```

### OpenAPI Specification

```yaml
openapi: 3.0.3
info:
  title: Dataset Generator Service Admin API
  description: Administrative endpoints for the Dataset Generator Service
  version: 0.1.0
  contact:
    name: Autapia Platform
servers:
  - url: http://localhost:20091
    description: Development server

paths:
  /admin/health:
    get:
      summary: Health Check
      description: Returns the health status of the service
      operationId: healthCheck
      tags:
        - Admin
      responses:
        '200':
          description: Service is healthy
          content:
            application/json:
              schema:
                type: object
                properties:
                  status:
                    type: string
                    example: "healthy"
                  service:
                    type: string
                    example: "dataset-generator-service"
                  version:
                    type: string
                    example: "0.1.0"
                  timestamp:
                    type: string
                    format: date-time
                    example: "2024-07-08T12:00:00Z"
              example:
                status: "healthy"
                service: "dataset-generator-service"
                version: "0.1.0"
                timestamp: "2024-07-08T12:00:00Z"

  /admin/reload:
    post:
      summary: Reload Configuration
      description: Reloads the service configuration from environment variables
      operationId: reloadConfig
      tags:
        - Admin
      responses:
        '200':
          description: Configuration reloaded successfully
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ReloadResponse'
              example:
                success: true
                message: "Dataset generator configuration reloaded successfully"
                timestamp: "2024-07-08T12:00:00Z"
        '500':
          description: Configuration reload failed
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ReloadResponse'
              example:
                success: false
                message: "Failed to reload configuration: Invalid environment variable"
                timestamp: "2024-07-08T12:00:00Z"

components:
  schemas:
    ReloadResponse:
      type: object
      required:
        - success
        - message
        - timestamp
      properties:
        success:
          type: boolean
          description: Whether the reload operation succeeded
        message:
          type: string
          description: Human-readable result message
        timestamp:
          type: string
          format: date-time
          description: ISO 8601 timestamp of the operation
```

### HTTP Endpoint Details

#### GET /admin/health
**Description:** Health check endpoint for monitoring service availability.

**Response Headers:**
- `Content-Type: application/json`

**Response Body:**
```json
{
  "status": "healthy",
  "service": "dataset-generator-service", 
  "version": "0.1.0",
  "timestamp": "2024-07-08T12:00:00Z"
}
```

**cURL Example:**
```bash
curl -X GET http://localhost:20091/admin/health
```

#### POST /admin/reload
**Description:** Reloads service configuration from environment variables without restarting.

**Configuration Variables Reloaded:**
- `GRPC_ADDR` - gRPC server bind address
- `VECTOR_SERVICE_ADDR` - Vector service URL  
- `EMBEDDING_SERVICE_ADDR` - Embedding service URL
- `CHAT_SERVICE_ADDR` - Chat service URL
- `OUTPUT_DIR` - Dataset output directory
- `MAX_CONCURRENT_JOBS` - Maximum concurrent generation jobs
- `DATABASE_URL` - PostgreSQL database connection string

**Response Headers:**
- `Content-Type: application/json`

**Success Response (200):**
```json
{
  "success": true,
  "message": "Dataset generator configuration reloaded successfully",
  "timestamp": "2024-07-08T12:00:00Z"
}
```

**Error Response (500):**
```json
{
  "success": false,
  "message": "Failed to load configuration: Invalid OUTPUT_DIR path",
  "timestamp": "2024-07-08T12:00:00Z"
}
```

**cURL Example:**
```bash
curl -X POST http://localhost:20091/admin/reload
```

## Database Schema

The service uses PostgreSQL with the following tables:

### datasets
```sql
CREATE TABLE datasets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR NOT NULL,
    description TEXT,
    file_path VARCHAR NOT NULL,
    record_count INTEGER NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### dataset_generation_jobs
```sql
CREATE TABLE dataset_generation_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR NOT NULL,
    input_config JSONB NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'pending',
    progress REAL NOT NULL DEFAULT 0.0,
    message TEXT,
    result_dataset_id UUID REFERENCES datasets(id),
    error_details TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);
```

### enhanced_dataset_jobs
```sql
CREATE TABLE enhanced_dataset_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_name VARCHAR NOT NULL,
    variations_per_use_case INTEGER NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'pending',
    progress REAL NOT NULL DEFAULT 0.0,
    message TEXT,
    total_examples INTEGER,
    total_endpoints INTEGER,
    services_covered TEXT[],
    output_path TEXT,
    error_details TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);
```

### job_statistics
```sql
CREATE TABLE job_statistics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL,
    job_type VARCHAR NOT NULL,
    execution_time_seconds REAL,
    memory_usage_mb REAL,
    cpu_usage_percent REAL,
    records_processed INTEGER,
    success_rate REAL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## Error Handling

### gRPC Status Codes
- `OK` (0) - Success
- `INVALID_ARGUMENT` (3) - Missing or invalid request parameters
- `NOT_FOUND` (5) - Dataset or job not found
- `RESOURCE_EXHAUSTED` (8) - Maximum concurrent jobs reached
- `INTERNAL` (13) - Internal server error

### HTTP Status Codes
- `200 OK` - Success
- `400 Bad Request` - Invalid request
- `404 Not Found` - Endpoint not found
- `500 Internal Server Error` - Server error

### Error Response Format
gRPC errors follow the standard tonic Status format:
```rust
Status::invalid_argument("Dataset configuration is required")
Status::resource_exhausted("Maximum concurrent jobs reached")
```

HTTP errors return JSON:
```json
{
  "success": false,
  "message": "Error description",
  "timestamp": "2024-07-08T12:00:00Z"
}
```

## Usage Examples

### Complete Workflow Example

1. **Generate Enhanced API Dataset:**
```bash
# Start enhanced API dataset generation
grpcurl -plaintext \
  -d '{"variations_per_use_case": 5}' \
  localhost:20090 \
  dataset_generator.DatasetGenerator/GenerateEnhancedApiDataset
```

2. **Monitor Progress:**
```bash
# Check job status (replace JOB_ID with actual ID)
grpcurl -plaintext \
  -d '{"job_id": "550e8400-e29b-41d4-a716-446655440000"}' \
  localhost:20090 \
  dataset_generator.DatasetGenerator/GetGenerationStatus
```

3. **List Generated Datasets:**
```bash
# List all datasets
grpcurl -plaintext \
  -d '{"page": 1, "page_size": 10}' \
  localhost:20090 \
  dataset_generator.DatasetGenerator/ListDatasets
```

### Configuration Management

1. **Check Service Health:**
```bash
curl http://localhost:20091/admin/health
```

2. **Reload Configuration:**
```bash
# Update environment variables, then reload
export OUTPUT_DIR="/new/dataset/path"
export MAX_CONCURRENT_JOBS=10
curl -X POST http://localhost:20091/admin/reload
```

## Client Integration

### Rust Client Example
```rust
use autapia_microservice_types::dataset_generator::{
    dataset_generator_client::DatasetGeneratorClient,
    GenerateEnhancedApiDatasetRequest,
};
use tonic::transport::Channel;

async fn generate_dataset() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DatasetGeneratorClient::connect("http://127.0.0.1:20090").await?;
    
    let request = tonic::Request::new(GenerateEnhancedApiDatasetRequest {
        variations_per_use_case: 3,
    });
    
    let response = client.generate_enhanced_api_dataset(request).await?;
    println!("Job ID: {}", response.into_inner().job_id);
    
    Ok(())
}
```

### Python Client Example
```python
import grpc
from generated import dataset_generator_pb2, dataset_generator_pb2_grpc

def generate_dataset():
    channel = grpc.insecure_channel('localhost:20090')
    stub = dataset_generator_pb2_grpc.DatasetGeneratorStub(channel)
    
    request = dataset_generator_pb2.GenerateEnhancedApiDatasetRequest(
        variations_per_use_case=3
    )
    
    response = stub.GenerateEnhancedApiDataset(request)
    print(f"Job ID: {response.job_id}")
```

## Performance Characteristics

### Throughput
- **Concurrent Jobs**: Up to 5 concurrent dataset generation jobs (configurable)
- **API Endpoints**: Processes 85+ API endpoints in enhanced mode
- **Generation Rate**: ~50-100 examples per minute depending on complexity

### Resource Usage
- **Memory**: ~500MB-2GB depending on dataset size
- **CPU**: Moderate usage during generation, minimal when idle
- **Disk**: Proportional to dataset size, typically 10-500MB per dataset
- **Network**: Moderate during generation (API calls to other services)

### Latency
- **Health Check**: < 10ms
- **Configuration Reload**: < 100ms
- **Dataset Generation**: 5-30 minutes depending on size and variations
- **Status Queries**: < 50ms

## Security Considerations

### Authentication
- Currently uses internal service communication (no external auth)
- Services communicate over HTTP within trusted network
- Future: Consider mTLS for production deployments

### Authorization
- No fine-grained permissions currently implemented
- Service-to-service communication trusted
- Admin endpoints accessible from localhost only

### Data Privacy
- Generated datasets may contain synthetic but realistic data
- Ensure compliance with data retention policies
- Consider anonymization for sensitive domains

## Monitoring and Observability

### Logging
- Structured logging with configurable levels
- Request/response logging for gRPC methods
- Performance metrics for generation operations

### Metrics (Future)
- Job success/failure rates
- Generation throughput
- Resource utilization
- Queue depths

### Health Monitoring
- HTTP health check endpoint
- Service dependency health
- Database connectivity status

## Migration and Deployment

### Environment Variables
```bash
# Required
DATABASE_URL=postgresql://postgres:root@localhost:5432/autapia

# Optional (with defaults)
GRPC_ADDR=0.0.0.0:20090
VECTOR_SERVICE_ADDR=http://127.0.0.1:20030
EMBEDDING_SERVICE_ADDR=http://127.0.0.1:20020
CHAT_SERVICE_ADDR=http://127.0.0.1:20010
OUTPUT_DIR=./datasets
MAX_CONCURRENT_JOBS=5
```

### Database Migration
```bash
# Run migrations on startup (automatic)
# Or manually:
cd services/dataset-generator-service
sqlx migrate run --database-url $DATABASE_URL
```

### Service Dependencies
- **PostgreSQL**: Database for job tracking and metadata
- **vector-service**: Vector operations and embeddings
- **embedding-service**: Text embedding generation
- **chat-service**: LLM interactions for dataset generation
- **settings-service**: Configuration management (optional) 