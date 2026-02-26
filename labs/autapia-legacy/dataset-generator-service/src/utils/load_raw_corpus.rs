use anyhow::{Result, anyhow};
use log::{debug, warn};
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

/// Resolve path relative to workspace root if it's a relative path
fn resolve_path(path: &str) -> Result<PathBuf> {
    let path = Path::new(path);
    
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        // For relative paths, resolve from workspace root
        // Detect if we're running from a service directory (contains "services/" in current dir)
        let current_dir = std::env::current_dir()?;
        let workspace_root = if current_dir.components().any(|c| c.as_os_str() == "services") {
            // We're in a service directory, go up to workspace root
            current_dir.ancestors()
                .find(|p| p.join("Cargo.toml").exists() && p.join("services").exists())
                .ok_or_else(|| anyhow!("Could not find workspace root"))?
                .to_path_buf()
        } else {
            // Assume we're already at workspace root
            current_dir
        };
        
        let resolved_path = workspace_root.join(path);
        debug!("Resolved path '{}' to '{}'", path.display(), resolved_path.display());
        Ok(resolved_path)
    }
}

pub async fn load_plain_text_files(path: &str) -> Result<Vec<(String, String)>> {
    let mut texts = Vec::new();
    let path = resolve_path(path)?;

    if path.is_file() {
        // Single file
        if let Some(extension) = path.extension() {
            if matches!(extension.to_str(), Some("txt") | Some("md")) {
                let content = fs::read_to_string(&path).await?;
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                texts.push((filename, content));
            }
        }
    } else if path.is_dir() {
        // Directory traversal
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            let entry_path = entry.path();
            if entry_path.is_file() {
                if let Some(extension) = entry_path.extension() {
                    if matches!(extension.to_str(), Some("txt") | Some("md")) {
                        match fs::read_to_string(entry_path).await {
                            Ok(content) => {
                                let filename = entry_path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                debug!("Loaded plain text file: {}", filename);
                                texts.push((filename, content));
                            }
                            Err(e) => {
                                warn!("Failed to read file {:?}: {}", entry_path, e);
                            }
                        }
                    }
                }
            }
        }
    } else {
        return Err(anyhow!("Path does not exist: {}", path.display()));
    }

    Ok(texts)
}

pub async fn load_jsonl_files(path: &str) -> Result<Vec<(String, String)>> {
    let mut texts = Vec::new();
    let path = resolve_path(path)?;

    if path.is_file() {
        // Single file
        if let Some(extension) = path.extension() {
            if extension.to_str() == Some("jsonl") {
                let content = fs::read_to_string(&path).await?;
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                // Parse JSONL and extract text content
                let extracted_texts = extract_text_from_jsonl(&content)?;
                for (i, text) in extracted_texts.into_iter().enumerate() {
                    texts.push((format!("{}_{}", filename, i), text));
                }
            }
        }
    } else if path.is_dir() {
        // Directory traversal
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            let entry_path = entry.path();
            if entry_path.is_file() {
                if let Some(extension) = entry_path.extension() {
                    if extension.to_str() == Some("jsonl") {
                        match fs::read_to_string(entry_path).await {
                            Ok(content) => {
                                let filename = entry_path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                
                                match extract_text_from_jsonl(&content) {
                                    Ok(extracted_texts) => {
                                        for (i, text) in extracted_texts.into_iter().enumerate() {
                                            texts.push((format!("{}_{}", filename, i), text));
                                        }
                                        debug!("Loaded JSONL file: {}", filename);
                                    }
                                    Err(e) => {
                                        warn!("Failed to parse JSONL file {:?}: {}", entry_path, e);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to read file {:?}: {}", entry_path, e);
                            }
                        }
                    }
                }
            }
        }
    } else {
        return Err(anyhow!("Path does not exist: {}", path.display()));
    }

    Ok(texts)
}

pub async fn load_csv_tsv_files(path: &str) -> Result<Vec<(String, String)>> {
    let mut texts = Vec::new();
    let path = resolve_path(path)?;

    if path.is_file() {
        // Single file
        if let Some(extension) = path.extension() {
            if matches!(extension.to_str(), Some("csv") | Some("tsv")) {
                let content = fs::read_to_string(&path).await?;
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                // Parse CSV/TSV and extract text content
                let delimiter = if extension.to_str() == Some("csv") { b',' } else { b'\t' };
                let extracted_texts = extract_text_from_csv(&content, delimiter)?;
                for (i, text) in extracted_texts.into_iter().enumerate() {
                    texts.push((format!("{}_{}", filename, i), text));
                }
            }
        }
    } else if path.is_dir() {
        // Directory traversal
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            let entry_path = entry.path();
            if entry_path.is_file() {
                if let Some(extension) = entry_path.extension() {
                    if matches!(extension.to_str(), Some("csv") | Some("tsv")) {
                        match fs::read_to_string(entry_path).await {
                            Ok(content) => {
                                let filename = entry_path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                
                                let delimiter = if extension.to_str() == Some("csv") { b',' } else { b'\t' };
                                match extract_text_from_csv(&content, delimiter) {
                                    Ok(extracted_texts) => {
                                        for (i, text) in extracted_texts.into_iter().enumerate() {
                                            texts.push((format!("{}_{}", filename, i), text));
                                        }
                                        debug!("Loaded CSV/TSV file: {}", filename);
                                    }
                                    Err(e) => {
                                        warn!("Failed to parse CSV/TSV file {:?}: {}", entry_path, e);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to read file {:?}: {}", entry_path, e);
                            }
                        }
                    }
                }
            }
        }
    } else {
        return Err(anyhow!("Path does not exist: {}", path.display()));
    }

    Ok(texts)
}

fn extract_text_from_jsonl(content: &str) -> Result<Vec<String>> {
    let mut texts = Vec::new();
    
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(json) => {
                // Try to extract text from common fields
                let text = extract_text_from_json(&json);
                if !text.is_empty() {
                    texts.push(text);
                }
            }
            Err(e) => {
                warn!("Failed to parse JSON line: {}", e);
            }
        }
    }
    
    Ok(texts)
}

fn extract_text_from_json(json: &serde_json::Value) -> String {
    // Try common text fields for raw text extraction
    let text_fields = ["text", "content", "body", "message", "description", "summary"];
    
    for field in &text_fields {
        if let Some(text) = json.get(field).and_then(|v| v.as_str()) {
            return text.to_string();
        }
    }
    
    // Try structured training dataset fields (for pre-existing datasets)
    let training_fields = ["input", "output", "question", "answer", "instruction", "response"];
    let mut training_parts = Vec::new();
    
    for field in &training_fields {
        if let Some(text) = json.get(field).and_then(|v| v.as_str()) {
            training_parts.push(format!("{}: {}", field, text));
        }
    }
    
    if !training_parts.is_empty() {
        return training_parts.join(" | ");
    }
    
    // If no specific fields found, try to concatenate string values
    let mut parts = Vec::new();
    collect_string_values(json, &mut parts);
    parts.join(" ")
}

fn collect_string_values(json: &serde_json::Value, parts: &mut Vec<String>) {
    match json {
        serde_json::Value::String(s) => parts.push(s.clone()),
        serde_json::Value::Array(arr) => {
            for item in arr {
                collect_string_values(item, parts);
            }
        }
        serde_json::Value::Object(obj) => {
            for value in obj.values() {
                collect_string_values(value, parts);
            }
        }
        _ => {}
    }
}

fn extract_text_from_csv(content: &str, delimiter: u8) -> Result<Vec<String>> {
    let mut texts = Vec::new();
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .from_reader(content.as_bytes());
    
    // Get headers to identify text columns
    let headers = reader.headers()?.clone();
    let text_columns: Vec<usize> = headers
        .iter()
        .enumerate()
        .filter(|(_, header)| {
            let header_lower = header.to_lowercase();
            header_lower.contains("text") || 
            header_lower.contains("content") || 
            header_lower.contains("body") ||
            header_lower.contains("message") ||
            header_lower.contains("description")
        })
        .map(|(i, _)| i)
        .collect();
    
    // If no specific text columns found, use all columns
    let columns_to_use = if text_columns.is_empty() {
        (0..headers.len()).collect()
    } else {
        text_columns
    };
    
    for result in reader.records() {
        let record = result?;
        let mut row_text = Vec::new();
        
        for &col_idx in &columns_to_use {
            if let Some(cell) = record.get(col_idx) {
                if !cell.trim().is_empty() {
                    row_text.push(cell.trim().to_string());
                }
            }
        }
        
        if !row_text.is_empty() {
            texts.push(row_text.join(" "));
        }
    }
    
    Ok(texts)
} 