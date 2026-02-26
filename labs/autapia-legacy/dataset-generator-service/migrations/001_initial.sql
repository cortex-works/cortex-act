-- Dataset Generator Service Database Schema
-- This migration creates tables for managing datasets and generation jobs

-- Table for storing dataset metadata
CREATE TABLE IF NOT EXISTS datasets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    file_path VARCHAR(512) NOT NULL,
    record_count INTEGER NOT NULL DEFAULT 0,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for performance
CREATE INDEX IF NOT EXISTS idx_datasets_status ON datasets(status);
CREATE INDEX IF NOT EXISTS idx_datasets_created_at ON datasets(created_at);

-- Table for storing dataset generation job metadata
CREATE TABLE IF NOT EXISTS dataset_generation_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    input_config JSONB NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    progress REAL NOT NULL DEFAULT 0.0,
    message TEXT,
    result_dataset_id UUID REFERENCES datasets(id),
    error_details TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_generation_jobs_status ON dataset_generation_jobs(status);
CREATE INDEX IF NOT EXISTS idx_generation_jobs_created_at ON dataset_generation_jobs(created_at);
CREATE INDEX IF NOT EXISTS idx_generation_jobs_result_dataset ON dataset_generation_jobs(result_dataset_id);

-- Table for storing enhanced dataset generation metadata
CREATE TABLE IF NOT EXISTS enhanced_dataset_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_name VARCHAR(255) NOT NULL,
    variations_per_use_case INTEGER NOT NULL DEFAULT 1,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    progress REAL NOT NULL DEFAULT 0.0,
    message TEXT,
    total_examples INTEGER DEFAULT 0,
    total_endpoints INTEGER DEFAULT 0,
    services_covered TEXT[] DEFAULT '{}',
    output_path VARCHAR(512),
    error_details TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

-- Indexes for enhanced dataset jobs
CREATE INDEX IF NOT EXISTS idx_enhanced_jobs_status ON enhanced_dataset_jobs(status);
CREATE INDEX IF NOT EXISTS idx_enhanced_jobs_created_at ON enhanced_dataset_jobs(created_at);

-- Table for tracking job execution statistics
CREATE TABLE IF NOT EXISTS job_statistics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL,
    job_type VARCHAR(100) NOT NULL, -- 'standard' or 'enhanced'
    execution_time_seconds REAL,
    memory_usage_mb REAL,
    cpu_usage_percent REAL,
    records_processed INTEGER DEFAULT 0,
    success_rate REAL DEFAULT 0.0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for statistics
CREATE INDEX IF NOT EXISTS idx_job_statistics_job_id ON job_statistics(job_id);
CREATE INDEX IF NOT EXISTS idx_job_statistics_type ON job_statistics(job_type);
CREATE INDEX IF NOT EXISTS idx_job_statistics_created_at ON job_statistics(created_at);

-- Function to update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Triggers to automatically update updated_at timestamps
CREATE TRIGGER update_datasets_updated_at 
    BEFORE UPDATE ON datasets 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_dataset_generation_jobs_updated_at 
    BEFORE UPDATE ON dataset_generation_jobs 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_enhanced_dataset_jobs_updated_at 
    BEFORE UPDATE ON enhanced_dataset_jobs 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column(); 