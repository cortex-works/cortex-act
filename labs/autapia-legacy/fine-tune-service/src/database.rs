use sqlx::{PgPool, postgres::PgPoolOptions};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use log::{debug, error, info};
use sqlx::Row;

use crate::error::{FineTuneError, Result};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct JobQueryResult {
    pub job_id: String,
    pub job_name: String,
    pub model_name: Option<String>,
    pub status: String,
    pub progress: Option<f32>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub training_config: Option<serde_json::Value>,
    pub dataset_config: Option<serde_json::Value>,
    pub output_config: Option<serde_json::Value>,
    pub model_type: Option<String>,
    pub fine_tuning_method: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobMetadata {
    pub job_id: String,
    pub job_name: String,
    pub model_name: String,
    pub status: String,
    pub progress: f32,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub logs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct JobSummary {
    pub job_id: String,
    pub job_name: String,
    pub model_name: Option<String>,
    pub status: String,
    pub progress: f32,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct FineTuneDatabase {
    pool: PgPool,
}

impl FineTuneDatabase {
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Initializing fine-tune database connection: {}", database_url);
        
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .map_err(|e| FineTuneError::Database(format!("Failed to connect to database: {}", e)))?;
        
        // DISABLED: Auto-migration can cause schema conflicts
        // sqlx::migrate!().run(&pool).await
        //     .map_err(|e| FineTuneError::Database(format!("Failed to run migrations: {}", e)))?;
        
        info!("Fine-tune database initialized successfully (auto-migration disabled)");
        Ok(Self { pool })
    }

    pub async fn health_check(&self) -> Result<()> {
        // Simple health check query
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| FineTuneError::Database(format!("Health check failed: {}", e)))?;
        Ok(())
    }

    pub async fn store_job_metadata(
        &self,
        job_id: &str,
        job_name: &str,
        model_name: &str,
        training_config: Option<serde_json::Value>,
        dataset_config: Option<serde_json::Value>,
        output_config: Option<serde_json::Value>,
        status: &str,
    ) -> Result<()> {
        debug!("Creating fine-tuning job: {}", job_id);

        // Use provided configs or create defaults
        let training_config = training_config.unwrap_or_else(|| serde_json::json!({"default": true}));
        let dataset_config = dataset_config.unwrap_or_else(|| serde_json::json!({"type": "local"}));
        let output_config = output_config.unwrap_or_else(|| serde_json::json!({"output_dir": "./fine_tuned_model"}));

        sqlx::query(
            r#"
            INSERT INTO fine_tuning_jobs (
                job_id, job_name, model_name, status, progress, 
                training_config, dataset_config, output_config, 
                model_type, fine_tuning_method, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)
            "#
        )
        .bind(job_id)
        .bind(job_name)
        .bind(model_name)
        .bind(status)
        .bind(0.0f32) // initial progress
        .bind(training_config)
        .bind(dataset_config)
        .bind(output_config)
        .bind(model_name) // model_type same as model_name for now
        .bind("LoRA") // default fine_tuning_method
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to create fine-tuning job in database: {}", e);
            FineTuneError::Database(format!("Failed to create fine-tuning job: {}", e))
        })?;

        debug!("Successfully created fine-tuning job: {}", job_id);
        Ok(())
    }

    pub async fn get_job_metadata(&self, job_id: &str) -> Result<JobMetadata> {
        debug!("Getting job metadata: {}", job_id);

        // Get the job  
        let job_row = sqlx::query_as::<_, JobQueryResult>(
            r#"
            SELECT job_id, job_name, model_name, status, progress, 
                   created_at, updated_at, training_config, dataset_config, 
                   output_config, model_type, fine_tuning_method
            FROM fine_tuning_jobs 
            WHERE job_id = $1
            "#
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get job metadata from database: {}", e);
            FineTuneError::Database(format!("Failed to get job metadata: {}", e))
        })?;

        let job_row = job_row.ok_or_else(|| FineTuneError::JobNotFound {
            job_id: job_id.to_string(),
        })?;

        // Get recent logs (limit to 10)  
        let log_rows = sqlx::query(
            r#"
            SELECT id, job_id, message, log_level, created_at
            FROM fine_tuning_job_logs 
            WHERE job_id = $1 
            ORDER BY created_at DESC 
            LIMIT 10
            "#
        )
        .bind(job_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get job logs from database: {}", e);
            FineTuneError::Database(format!("Failed to get job logs: {}", e))
        })?;

        let logs: Vec<String> = log_rows.into_iter().map(|log| {
            log.try_get::<String, _>("message").unwrap_or_default()
        }).collect();

        let metadata = JobMetadata {
            job_id: job_row.job_id,
            job_name: job_row.job_name,
            model_name: job_row.model_name.unwrap_or_else(|| "unknown".to_string()),
            status: job_row.status,
            progress: job_row.progress.unwrap_or(0.0),
            created_at: job_row.created_at.unwrap_or_else(|| Utc::now()).to_rfc3339(),
            updated_at: job_row.updated_at.map(|dt| dt.to_rfc3339()),
            logs,
        };

        debug!("Successfully retrieved job metadata: {}", job_id);
        Ok(metadata)
    }

    pub async fn list_jobs(&self, limit: Option<i32>) -> Result<Vec<JobSummary>> {
        debug!("Listing fine-tune jobs");

        let limit = limit.unwrap_or(100);

        let job_rows = sqlx::query_as::<_, JobSummary>(
            r#"
            SELECT job_id, job_name, model_name, status, progress, created_at
            FROM fine_tuning_jobs 
            ORDER BY created_at DESC 
            LIMIT $1
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to list jobs from database: {}", e);
            FineTuneError::Database(format!("Failed to list jobs: {}", e))
        })?;

        debug!("Successfully listed {} fine-tune jobs", job_rows.len());
        Ok(job_rows)
    }

    pub async fn delete_job(&self, job_id: &str) -> Result<()> {
        debug!("Deleting fine-tune job: {}", job_id);

        // Delete logs first (foreign key constraint)
        sqlx::query("DELETE FROM fine_tuning_job_logs WHERE job_id = $1")
            .bind(job_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to delete job logs from database: {}", e);
            FineTuneError::Database(format!("Failed to delete job logs: {}", e))
        })?;

        // Delete the job
        let result = sqlx::query!(
            "DELETE FROM fine_tuning_jobs WHERE job_id = $1",
            job_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to delete job from database: {}", e);
            FineTuneError::Database(format!("Failed to delete job: {}", e))
        })?;

        if result.rows_affected() == 0 {
            return Err(FineTuneError::JobNotFound {
                job_id: job_id.to_string(),
            });
        }

        debug!("Successfully deleted fine-tune job: {}", job_id);
        Ok(())
    }

    pub async fn update_job_status(
        &self,
        job_id: &str,
        status: &str,
        progress: f32,
        logs: &[String],
    ) -> Result<()> {
        debug!("Updating job status for {}: {} ({}%)", job_id, status, progress * 100.0);

        // Update the job status and progress
        sqlx::query!(
            r#"
            UPDATE fine_tuning_jobs 
            SET status = $1, progress = $2, updated_at = $3
            WHERE job_id = $4
            "#,
            status,
            progress,
            Utc::now(),
            job_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to update job status in database: {}", e);
            FineTuneError::Database(format!("Failed to update job status: {}", e))
        })?;

        // Add new log entries
        for log_message in logs {
            sqlx::query(
                r#"
                INSERT INTO fine_tuning_job_logs (job_id, message, log_level, created_at)
                VALUES ($1, $2, $3, $4)
                "#
            )
            .bind(job_id)
            .bind(log_message)
            .bind("INFO") // default log level
            .bind(Utc::now())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to insert job log: {}", e);
                FineTuneError::Database(format!("Failed to insert job log: {}", e))
            })?;
        }

        debug!("Successfully updated job status for: {}", job_id);
        Ok(())
    }
}
