-- Initial migration for Fine-Tune service

-- Fine-tuning jobs table
CREATE TABLE IF NOT EXISTS fine_tuning_jobs (
    job_id VARCHAR PRIMARY KEY,
    job_name VARCHAR NOT NULL,
    model_name VARCHAR,
    status VARCHAR NOT NULL DEFAULT 'PENDING',
    progress REAL DEFAULT 0.0,
    training_config JSONB,
    dataset_config JSONB,
    output_config JSONB,
    model_type VARCHAR,
    fine_tuning_method VARCHAR DEFAULT 'LoRA',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Fine-tuning job logs table
CREATE TABLE IF NOT EXISTS fine_tuning_job_logs (
    id SERIAL PRIMARY KEY,
    job_id VARCHAR REFERENCES fine_tuning_jobs(job_id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    log_level VARCHAR NOT NULL DEFAULT 'INFO',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_fine_tuning_jobs_status ON fine_tuning_jobs(status);
CREATE INDEX IF NOT EXISTS idx_fine_tuning_jobs_created_at ON fine_tuning_jobs(created_at);
CREATE INDEX IF NOT EXISTS idx_fine_tuning_job_logs_job_id ON fine_tuning_job_logs(job_id);
CREATE INDEX IF NOT EXISTS idx_fine_tuning_job_logs_created_at ON fine_tuning_job_logs(created_at); 