use autapia_database_client::{DatabasePool, DatabaseConfig};
use chrono::{DateTime, Utc};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::error::{DatasetError, Result};
use crate::types::{JobStatus, DatasetJob};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Dataset {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub file_path: String,
    pub record_count: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct DatasetGenerationJob {
    pub id: Uuid,
    pub name: String,
    pub input_config: serde_json::Value,
    pub status: String,
    pub progress: f32,
    pub message: Option<String>,
    pub result_dataset_id: Option<Uuid>,
    pub error_details: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct EnhancedDatasetJob {
    pub id: Uuid,
    pub job_name: String,
    pub variations_per_use_case: i32,
    pub status: String,
    pub progress: f32,
    pub message: Option<String>,
    pub total_examples: Option<i32>,
    pub total_endpoints: Option<i32>,
    pub services_covered: Vec<String>,
    pub output_path: Option<String>,
    pub error_details: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct JobStatistics {
    pub id: Uuid,
    pub job_id: Uuid,
    pub job_type: String,
    pub execution_time_seconds: Option<f32>,
    pub memory_usage_mb: Option<f32>,
    pub cpu_usage_percent: Option<f32>,
    pub records_processed: Option<i32>,
    pub success_rate: Option<f32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDataset {
    pub name: String,
    pub description: Option<String>,
    pub file_path: String,
    pub record_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDatasetJob {
    pub name: String,
    pub input_config: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateEnhancedDatasetJob {
    pub job_name: String,
    pub variations_per_use_case: i32,
}

#[derive(Clone)]
pub struct DatasetDatabase {
    pool: DatabasePool,
}

impl DatasetDatabase {
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Initializing Dataset Generator database connection: {}", database_url);
        
        let config = DatabaseConfig {
            url: database_url.to_string(),
            ..Default::default()
        };
        
        let pool = DatabasePool::new("dataset-generator-service", config).await
            .map_err(|e| DatasetError::Database(format!("Failed to create database pool: {}", e)))?;
        
        // Run service-specific migrations
        info!("Running dataset generator service migrations...");
        autapia_database_client::migrations::run_migrations(pool.pool(), "dataset-generator-service").await
            .map_err(|e| DatasetError::Database(format!("Failed to run migrations: {}", e)))?;
        
        info!("Dataset Generator database initialized successfully");
        Ok(Self { pool })
    }

    pub async fn health_check(&self) -> Result<()> {
        self.pool.health_check().await
            .map_err(|e| DatasetError::Database(format!("Health check failed: {}", e)))?;
        Ok(())
    }

    // Dataset management
    pub async fn create_dataset(&self, dataset: &CreateDataset) -> Result<Dataset> {
        debug!("Creating dataset: {}", dataset.name);
        
        let dataset_id = Uuid::new_v4();
        let now = Utc::now();
        
        sqlx::query(
            r#"
            INSERT INTO datasets (id, name, description, file_path, record_count, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, 'active', $6, $6)
            "#
        )
        .bind(dataset_id)
        .bind(&dataset.name)
        .bind(&dataset.description)
        .bind(&dataset.file_path)
        .bind(dataset.record_count)
        .bind(now)
        .execute(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to create dataset: {}", e);
            DatasetError::Database(format!("Failed to create dataset: {}", e))
        })?;

        let created_dataset = Dataset {
            id: dataset_id,
            name: dataset.name.clone(),
            description: dataset.description.clone(),
            file_path: dataset.file_path.clone(),
            record_count: dataset.record_count,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        };

        debug!("Successfully created dataset with ID: {}", dataset_id);
        Ok(created_dataset)
    }

    pub async fn get_dataset(&self, dataset_id: &Uuid) -> Result<Option<Dataset>> {
        debug!("Getting dataset: {}", dataset_id);

        let dataset = sqlx::query_as::<_, Dataset>(
            r#"
            SELECT id, name, description, file_path, record_count, status, created_at, updated_at 
            FROM datasets 
            WHERE id = $1 AND status = 'active'
            "#
        )
        .bind(dataset_id)
        .fetch_optional(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to get dataset: {}", e);
            DatasetError::Database(format!("Failed to get dataset: {}", e))
        })?;

        Ok(dataset)
    }

    pub async fn list_datasets(&self, limit: i64, offset: i64) -> Result<Vec<Dataset>> {
        debug!("Listing datasets (limit: {}, offset: {})", limit, offset);

        let datasets = sqlx::query_as::<_, Dataset>(
            r#"
            SELECT id, name, description, file_path, record_count, status, created_at, updated_at 
            FROM datasets 
            WHERE status = 'active'
            ORDER BY created_at DESC 
            LIMIT $1 OFFSET $2
            "#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to list datasets: {}", e);
            DatasetError::Database(format!("Failed to list datasets: {}", e))
        })?;

        debug!("Retrieved {} datasets", datasets.len());
        Ok(datasets)
    }

    pub async fn delete_dataset(&self, dataset_id: &Uuid) -> Result<bool> {
        debug!("Deleting dataset: {}", dataset_id);

        let result = sqlx::query(
            "UPDATE datasets SET status = 'deleted', updated_at = $1 WHERE id = $2"
        )
        .bind(Utc::now())
        .bind(dataset_id)
        .execute(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to delete dataset: {}", e);
            DatasetError::Database(format!("Failed to delete dataset: {}", e))
        })?;

        Ok(result.rows_affected() > 0)
    }

    // Enhanced dataset job management
    pub async fn create_enhanced_dataset_job(&self, job: &CreateEnhancedDatasetJob) -> Result<Uuid> {
        debug!("Creating enhanced dataset generation job: {}", job.job_name);
        
        let job_id = Uuid::new_v4();
        let now = Utc::now();
        
        sqlx::query(
            r#"
            INSERT INTO enhanced_dataset_jobs (id, job_name, variations_per_use_case, status, progress, created_at, updated_at)
            VALUES ($1, $2, $3, 'pending', 0.0, $4, $4)
            "#
        )
        .bind(job_id)
        .bind(&job.job_name)
        .bind(job.variations_per_use_case)
        .bind(now)
        .execute(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to create enhanced dataset job: {}", e);
            DatasetError::Database(format!("Failed to create enhanced dataset job: {}", e))
        })?;

        debug!("Successfully created enhanced dataset job with ID: {}", job_id);
        Ok(job_id)
    }

    pub async fn get_enhanced_dataset_job(&self, job_id: &Uuid) -> Result<Option<EnhancedDatasetJob>> {
        debug!("Getting enhanced dataset job: {}", job_id);

        let job = sqlx::query_as::<_, EnhancedDatasetJob>(
            r#"
            SELECT id, job_name, variations_per_use_case, status, progress, message, 
                   total_examples, total_endpoints, services_covered, output_path, 
                   error_details, created_at, updated_at, completed_at 
            FROM enhanced_dataset_jobs 
            WHERE id = $1
            "#
        )
        .bind(job_id)
        .fetch_optional(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to get enhanced dataset job: {}", e);
            DatasetError::Database(format!("Failed to get enhanced dataset job: {}", e))
        })?;

        Ok(job)
    }

    pub async fn update_enhanced_job_status(&self, job_id: &Uuid, status: &str, progress: f32, message: Option<&str>) -> Result<bool> {
        debug!("Updating enhanced job {} status to: {} (progress: {})", job_id, status, progress);

        let completed_at = if status == "completed" || status == "failed" {
            Some(Utc::now())
        } else {
            None
        };

        let result = sqlx::query(
            r#"
            UPDATE enhanced_dataset_jobs 
            SET status = $1, progress = $2, message = $3, updated_at = $4, completed_at = $5
            WHERE id = $6
            "#
        )
        .bind(status)
        .bind(progress)
        .bind(message)
        .bind(Utc::now())
        .bind(completed_at)
        .bind(job_id)
        .execute(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to update enhanced job status: {}", e);
            DatasetError::Database(format!("Failed to update enhanced job status: {}", e))
        })?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn complete_enhanced_job(&self, job_id: &Uuid, total_examples: i32, total_endpoints: i32, services_covered: &[String], output_path: &str) -> Result<bool> {
        debug!("Completing enhanced job {} with {} examples covering {} endpoints", job_id, total_examples, total_endpoints);

        let result = sqlx::query(
            r#"
            UPDATE enhanced_dataset_jobs 
            SET status = 'completed', progress = 1.0, total_examples = $1, total_endpoints = $2, 
                services_covered = $3, output_path = $4, completed_at = $5, updated_at = $5
            WHERE id = $6
            "#
        )
        .bind(total_examples)
        .bind(total_endpoints)
        .bind(services_covered)
        .bind(output_path)
        .bind(Utc::now())
        .bind(job_id)
        .execute(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to complete enhanced job: {}", e);
            DatasetError::Database(format!("Failed to complete enhanced job: {}", e))
        })?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn fail_enhanced_job(&self, job_id: &Uuid, error_details: &str) -> Result<bool> {
        debug!("Failing enhanced job {} with error: {}", job_id, error_details);

        let result = sqlx::query(
            r#"
            UPDATE enhanced_dataset_jobs 
            SET status = 'failed', error_details = $1, completed_at = $2, updated_at = $2
            WHERE id = $3
            "#
        )
        .bind(error_details)
        .bind(Utc::now())
        .bind(job_id)
        .execute(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to fail enhanced job: {}", e);
            DatasetError::Database(format!("Failed to fail enhanced job: {}", e))
        })?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn list_enhanced_dataset_jobs(&self, limit: i64, offset: i64) -> Result<Vec<EnhancedDatasetJob>> {
        debug!("Listing enhanced dataset jobs (limit: {}, offset: {})", limit, offset);

        let jobs = sqlx::query_as::<_, EnhancedDatasetJob>(
            r#"
            SELECT id, job_name, variations_per_use_case, status, progress, message, 
                   total_examples, total_endpoints, services_covered, output_path, 
                   error_details, created_at, updated_at, completed_at 
            FROM enhanced_dataset_jobs 
            ORDER BY created_at DESC 
            LIMIT $1 OFFSET $2
            "#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to list enhanced dataset jobs: {}", e);
            DatasetError::Database(format!("Failed to list enhanced dataset jobs: {}", e))
        })?;

        debug!("Retrieved {} enhanced dataset jobs", jobs.len());
        Ok(jobs)
    }

    // Standard dataset generation job management (existing methods updated)
    pub async fn create_generation_job(&self, job: &CreateDatasetJob) -> Result<Uuid> {
        debug!("Creating dataset generation job: {}", job.name);
        
        let job_id = Uuid::new_v4();
        let now = Utc::now();
        
        sqlx::query(
            r#"
            INSERT INTO dataset_generation_jobs (id, name, input_config, status, progress, created_at, updated_at)
            VALUES ($1, $2, $3, 'pending', 0.0, $4, $4)
            "#
        )
        .bind(job_id)
        .bind(&job.name)
        .bind(&job.input_config)
        .bind(now)
        .execute(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to create generation job: {}", e);
            DatasetError::Database(format!("Failed to create generation job: {}", e))
        })?;

        debug!("Successfully created generation job with ID: {}", job_id);
        Ok(job_id)
    }

    pub async fn get_generation_job(&self, job_id: &Uuid) -> Result<Option<DatasetGenerationJob>> {
        debug!("Getting generation job: {}", job_id);

        let job = sqlx::query_as::<_, DatasetGenerationJob>(
            r#"
            SELECT id, name, input_config, status, progress, message, result_dataset_id, 
                   error_details, created_at, updated_at, completed_at 
            FROM dataset_generation_jobs 
            WHERE id = $1
            "#
        )
        .bind(job_id)
        .fetch_optional(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to get generation job: {}", e);
            DatasetError::Database(format!("Failed to get generation job: {}", e))
        })?;

        Ok(job)
    }

    pub async fn update_job_status(&self, job_id: &Uuid, status: &str, progress: f32, message: Option<&str>) -> Result<bool> {
        debug!("Updating job {} status to: {} (progress: {})", job_id, status, progress);

        let completed_at = if status == "completed" || status == "failed" {
            Some(Utc::now())
        } else {
            None
        };

        let result = sqlx::query(
            r#"
            UPDATE dataset_generation_jobs 
            SET status = $1, progress = $2, message = $3, updated_at = $4, completed_at = $5
            WHERE id = $6
            "#
        )
        .bind(status)
        .bind(progress)
        .bind(message)
        .bind(Utc::now())
        .bind(completed_at)
        .bind(job_id)
        .execute(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to update job status: {}", e);
            DatasetError::Database(format!("Failed to update job status: {}", e))
        })?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn complete_job(&self, job_id: &Uuid, dataset_id: &Uuid) -> Result<bool> {
        debug!("Completing job {} with dataset {}", job_id, dataset_id);

        let result = sqlx::query(
            r#"
            UPDATE dataset_generation_jobs 
            SET status = 'completed', progress = 1.0, result_dataset_id = $1, completed_at = $2, updated_at = $2
            WHERE id = $3
            "#
        )
        .bind(dataset_id)
        .bind(Utc::now())
        .bind(job_id)
        .execute(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to complete job: {}", e);
            DatasetError::Database(format!("Failed to complete job: {}", e))
        })?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn list_generation_jobs(&self, limit: i64, offset: i64) -> Result<Vec<DatasetGenerationJob>> {
        debug!("Listing generation jobs (limit: {}, offset: {})", limit, offset);

        let jobs = sqlx::query_as::<_, DatasetGenerationJob>(
            r#"
            SELECT id, name, input_config, status, progress, message, result_dataset_id, 
                   error_details, created_at, updated_at, completed_at 
            FROM dataset_generation_jobs 
            ORDER BY created_at DESC 
            LIMIT $1 OFFSET $2
            "#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to list generation jobs: {}", e);
            DatasetError::Database(format!("Failed to list generation jobs: {}", e))
        })?;

        debug!("Retrieved {} generation jobs", jobs.len());
        Ok(jobs)
    }

    pub async fn get_active_jobs_count(&self) -> Result<i64> {
        debug!("Getting active jobs count");

        let row = sqlx::query(
            r#"
            SELECT 
                (SELECT COUNT(*) FROM dataset_generation_jobs WHERE status IN ('pending', 'processing')) +
                (SELECT COUNT(*) FROM enhanced_dataset_jobs WHERE status IN ('pending', 'processing'))
                as count
            "#
        )
        .fetch_one(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to get active jobs count: {}", e);
            DatasetError::Database(format!("Failed to get active jobs count: {}", e))
        })?;

        let count: i64 = row.get("count");
        Ok(count)
    }

    // Job Statistics Management
    pub async fn record_job_statistics(&self, job_id: &Uuid, job_type: &str, stats: &JobStatistics) -> Result<()> {
        debug!("Recording statistics for job {} of type {}", job_id, job_type);

        sqlx::query(
            r#"
            INSERT INTO job_statistics (job_id, job_type, execution_time_seconds, memory_usage_mb, 
                                      cpu_usage_percent, records_processed, success_rate, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#
        )
        .bind(job_id)
        .bind(job_type)
        .bind(stats.execution_time_seconds)
        .bind(stats.memory_usage_mb)
        .bind(stats.cpu_usage_percent)
        .bind(stats.records_processed)
        .bind(stats.success_rate)
        .bind(Utc::now())
        .execute(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to record job statistics: {}", e);
            DatasetError::Database(format!("Failed to record job statistics: {}", e))
        })?;

        debug!("Successfully recorded statistics for job {}", job_id);
        Ok(())
    }

    // Statistics
    pub async fn get_processing_statistics(&self) -> Result<serde_json::Value> {
        debug!("Getting processing statistics");

        let stats = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_jobs,
                COUNT(*) FILTER (WHERE status = 'completed') as completed_jobs,
                COUNT(*) FILTER (WHERE status = 'failed') as failed_jobs,
                COUNT(*) FILTER (WHERE status = 'pending') as pending_jobs,
                COUNT(*) FILTER (WHERE status = 'processing') as processing_jobs,
                AVG(CASE WHEN status = 'completed' THEN progress ELSE NULL END) as avg_progress
            FROM (
                SELECT status, progress FROM dataset_generation_jobs
                UNION ALL
                SELECT status, progress FROM enhanced_dataset_jobs
            ) combined_jobs
            "#
        )
        .fetch_one(self.pool.pool())
        .await
        .map_err(|e| {
            error!("Failed to get processing statistics: {}", e);
            DatasetError::Database(format!("Failed to get processing statistics: {}", e))
        })?;

        let result = serde_json::json!({
            "total_jobs": stats.get::<i64, _>("total_jobs"),
            "completed_jobs": stats.get::<i64, _>("completed_jobs"),
            "failed_jobs": stats.get::<i64, _>("failed_jobs"),
            "pending_jobs": stats.get::<i64, _>("pending_jobs"),
            "processing_jobs": stats.get::<i64, _>("processing_jobs"),
            "average_progress": stats.get::<Option<f64>, _>("avg_progress").unwrap_or(0.0)
        });

        Ok(result)
    }
} 