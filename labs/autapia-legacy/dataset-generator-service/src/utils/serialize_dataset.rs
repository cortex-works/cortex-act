use crate::types::DatasetSample;
use anyhow::Result;
use serde_json;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

pub async fn serialize_to_jsonl(samples: &[DatasetSample], output_path: &Path) -> Result<u64> {
    let file = File::create(output_path).await?;
    let mut writer = BufWriter::new(file);
    let mut total_bytes = 0u64;

    for sample in samples {
        let json_line = serde_json::to_string(sample)?;
        let line_bytes = json_line.as_bytes();
        writer.write_all(line_bytes).await?;
        writer.write_all(b"\n").await?;
        total_bytes += line_bytes.len() as u64 + 1; // +1 for newline
    }

    writer.flush().await?;
    Ok(total_bytes)
}

pub async fn serialize_to_csv(samples: &[DatasetSample], output_path: &Path) -> Result<u64> {
    let file = File::create(output_path).await?;
    let mut writer = BufWriter::new(file);
    let mut total_bytes = 0u64;

    // Write CSV header
    let header = "id,input,output,instruction,context,quality_score,split\n";
    writer.write_all(header.as_bytes()).await?;
    total_bytes += header.len() as u64;

    for sample in samples {
        let instruction = sample.instruction.as_deref().unwrap_or("");
        let context = sample.context.as_deref().unwrap_or("");
        let split = format!("{:?}", sample.split);
        
        // Escape CSV fields
        let input = escape_csv_field(&sample.input);
        let output = escape_csv_field(&sample.output);
        let instruction_escaped = escape_csv_field(instruction);
        let context_escaped = escape_csv_field(context);

        let csv_line = format!(
            "{},{},{},{},{},{},{}\n",
            sample.id,
            input,
            output,
            instruction_escaped,
            context_escaped,
            sample.quality_score,
            split
        );

        writer.write_all(csv_line.as_bytes()).await?;
        total_bytes += csv_line.len() as u64;
    }

    writer.flush().await?;
    Ok(total_bytes)
}

pub async fn serialize_to_parquet(samples: &[DatasetSample], output_path: &Path) -> Result<u64> {
    // For now, we'll use Polars to write Parquet files
    use polars::prelude::*;

    let mut ids = Vec::new();
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut instructions = Vec::new();
    let mut contexts = Vec::new();
    let mut quality_scores = Vec::new();
    let mut splits = Vec::new();

    for sample in samples {
        ids.push(sample.id.clone());
        inputs.push(sample.input.clone());
        outputs.push(sample.output.clone());
        instructions.push(sample.instruction.clone());
        contexts.push(sample.context.clone());
        quality_scores.push(sample.quality_score);
        splits.push(format!("{:?}", sample.split));
    }

    let df = df! [
        "id" => ids,
        "input" => inputs,
        "output" => outputs,
        "instruction" => instructions,
        "context" => contexts,
        "quality_score" => quality_scores,
        "split" => splits,
    ]?;

    let mut file = std::fs::File::create(output_path)?;
    ParquetWriter::new(&mut file).finish(&mut df.clone())?;

    // Get file size
    let metadata = tokio::fs::metadata(output_path).await?;
    Ok(metadata.len())
}

fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

// Helper function to create training format variations
pub fn create_training_formats(samples: &[DatasetSample]) -> Vec<serde_json::Value> {
    samples
        .iter()
        .map(|sample| {
            let mut training_sample = serde_json::json!({
                "input": sample.input,
                "output": sample.output,
            });

            // Add instruction if available
            if let Some(instruction) = &sample.instruction {
                training_sample["instruction"] = serde_json::Value::String(instruction.clone());
            }

            // Add context if available
            if let Some(context) = &sample.context {
                training_sample["context"] = serde_json::Value::String(context.clone());
            }

            // Add metadata
            training_sample["metadata"] = serde_json::to_value(&sample.metadata).unwrap_or_default();
            training_sample["quality_score"] = serde_json::Value::Number(
                serde_json::Number::from_f64(sample.quality_score as f64).unwrap_or_else(|| serde_json::Number::from(0))
            );

            training_sample
        })
        .collect()
}

// Create different dataset format variations
pub async fn serialize_multiple_formats(
    samples: &[DatasetSample],
    base_path: &Path,
    formats: &[&str],
) -> Result<Vec<(String, u64)>> {
    let mut results = Vec::new();

    for format in formats {
        let file_extension = match *format {
            "jsonl" => "jsonl",
            "csv" => "csv",
            "parquet" => "parquet",
            _ => continue,
        };

        let output_path = base_path.with_extension(file_extension);
        
        let file_size = match *format {
            "jsonl" => serialize_to_jsonl(samples, &output_path).await?,
            "csv" => serialize_to_csv(samples, &output_path).await?,
            "parquet" => serialize_to_parquet(samples, &output_path).await?,
            _ => continue,
        };

        results.push((output_path.to_string_lossy().to_string(), file_size));
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DatasetSplit};
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn create_test_sample() -> DatasetSample {
        DatasetSample {
            id: "test_1".to_string(),
            input: "What is the capital of France?".to_string(),
            output: "The capital of France is Paris.".to_string(),
            instruction: Some("Answer the question".to_string()),
            context: Some("Geography context".to_string()),
            metadata: HashMap::new(),
            quality_score: 0.9,
            split: DatasetSplit::Train,
        }
    }

    #[tokio::test]
    async fn test_serialize_to_jsonl() {
        let samples = vec![create_test_sample()];
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("test.jsonl");

        let file_size = serialize_to_jsonl(&samples, &output_path).await.unwrap();
        assert!(file_size > 0);

        let content = tokio::fs::read_to_string(&output_path).await.unwrap();
        assert!(content.contains("What is the capital of France?"));
    }

    #[tokio::test]
    async fn test_serialize_to_csv() {
        let samples = vec![create_test_sample()];
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("test.csv");

        let file_size = serialize_to_csv(&samples, &output_path).await.unwrap();
        assert!(file_size > 0);

        let content = tokio::fs::read_to_string(&output_path).await.unwrap();
        assert!(content.contains("id,input,output"));
        assert!(content.contains("What is the capital of France?"));
    }

    #[test]
    fn test_escape_csv_field() {
        assert_eq!(escape_csv_field("simple"), "simple");
        assert_eq!(escape_csv_field("with,comma"), "\"with,comma\"");
        assert_eq!(escape_csv_field("with\"quote"), "\"with\"\"quote\"");
    }
} 