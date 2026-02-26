use std::path::Path;
use anyhow::{Context, Result};
use crate::act::auto_healer::try_auto_heal;

// ─── Symbol type (local to cortex-act, independent of CortexAST inspector) ───

pub struct Symbol {
    pub name:       String,
    pub kind:       String,
    pub start_byte: usize,
    pub end_byte:   usize,
}

pub struct AstEdit {
    /// e.g. "class:Auth" or "function:login" or just the bare identifier "login"
    pub target: String,
    pub action: String, // "replace", "delete"
    pub code:   String,
}

// ─── Tree-sitter based symbol extraction for core-3 languages ─────────────────

/// Extract named symbols (functions, classes, structs, impls) from source using
/// Tree-sitter for Rust and simple regex for other languages.
pub fn extract_symbols(file_path: &Path, source: &str) -> Vec<Symbol> {
    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "rs" => extract_via_tree_sitter_rust(source),
        _    => extract_via_regex(source),
    }
}

fn extract_via_tree_sitter_rust(source: &str) -> Vec<Symbol> {
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&tree_sitter_rust::language().into()).is_err() {
        return Vec::new();
    }
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None    => return Vec::new(),
    };

    let mut symbols = Vec::new();
    let root = tree.root_node();
    collect_rust_symbols(root, source, &mut symbols);
    symbols
}

fn get_name_child<'a>(node: tree_sitter::Node<'a>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" || child.kind() == "type_identifier" {
            return source.get(child.byte_range()).map(|s| s.to_string());
        }
    }
    None
}

fn collect_rust_symbols(node: tree_sitter::Node, source: &str, out: &mut Vec<Symbol>) {
    let kind = match node.kind() {
        "function_item"    => Some("function"),
        "struct_item"      => Some("struct"),
        "enum_item"        => Some("enum"),
        "impl_item"        => Some("impl"),
        "trait_item"       => Some("trait"),
        "mod_item"         => Some("mod"),
        _                  => None,
    };
    if let Some(k) = kind {
        if let Some(name) = get_name_child(node, source) {
            out.push(Symbol {
                name,
                kind:       k.to_string(),
                start_byte: node.start_byte(),
                end_byte:   node.end_byte(),
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_rust_symbols(child, source, out);
    }
}

fn extract_via_regex(source: &str) -> Vec<Symbol> {
    // Cheap heuristic extractor for non-Rust files
    let patterns: &[(&str, &str)] = &[
        // Rust / general
        (r"(?m)^(?:pub\s+)?(?:async\s+)?fn\s+(\w+)", "function"),
        (r"(?m)^(?:pub\s+)?struct\s+(\w+)", "struct"),
        (r"(?m)^(?:pub\s+)?enum\s+(\w+)", "enum"),
        // TS / JS
        (r"(?m)^(?:export\s+)?(?:default\s+)?(?:async\s+)?function\s+(\w+)", "function"),
        (r"(?m)^(?:export\s+)?(?:default\s+)?class\s+(\w+)", "class"),
        (r"(?m)^(?:export\s+)?interface\s+(\w+)", "interface"),
        // Python
        (r"(?m)^def\s+(\w+)", "function"),
        (r"(?m)^class\s+(\w+)", "class"),
        // Go
        (r"(?m)^func\s+(?:\([^)]+\)\s+)?(\w+)", "function"),
        (r"(?m)^type\s+(\w+)\s+struct", "struct"),
        // PHP
        (r"(?m)^(?:public\s+|private\s+|protected\s+)?(?:static\s+)?function\s+(\w+)", "function"),
        (r"(?m)^(?:abstract\s+|final\s+)?class\s+(\w+)", "class"),
        // C# / Java / C++ (Basic signature match)
        (r"(?m)^(?:public\s+|private\s+|protected\s+|internal\s+)?(?:static\s+|async\s+|virtual\s+|override\s+)?(?:[\w<>,\[\]]+\s+)(\w+)\s*\(", "function"),
    ];
    let mut symbols = Vec::new();
    for (pat, kind) in patterns {
        if let Ok(re) = regex::Regex::new(pat) {
            for cap in re.captures_iter(source) {
                if let Some(m) = cap.get(1) {
                    // Find full "line" range from pattern match
                    let name_start = m.start();
                    // Walk forward from name_start to find the end of the block safely
                    let suffix = &source[name_start..];
                    let offset = suffix.char_indices().nth(500).map(|(i, _)| i).unwrap_or(suffix.len());
                    let surrogate_end = name_start + offset;
                    symbols.push(Symbol {
                        name:       m.as_str().to_string(),
                        kind:       kind.to_string(),
                        start_byte: name_start,
                        end_byte:   surrogate_end,
                    });
                }
            }
        }
    }
    symbols
}

// ─── Core AST Editor ──────────────────────────────────────────────────────────

pub fn apply_ast_edits(
    file_path: &Path,
    edits:     Vec<AstEdit>,
    llm_url:   Option<&str>,
) -> Result<String> {
    // 0. Permission Guard
    check_write_permission(file_path)?;

    let source_bytes = std::fs::read(file_path).context("Failed to read original source")?;
    let mut current_source = String::from_utf8_lossy(&source_bytes).into_owned();

    // 1. Gather targeted byte ranges with symbol extractor
    let mut operations = Vec::new();
    let symbols = extract_symbols(file_path, &current_source);

    for edit in edits {
        let sym = symbols.iter().find(|s| {
            let full_name = format!("{}:{}", s.kind, s.name);
            edit.target == full_name || edit.target == s.name
        });

        if let Some(s) = sym {
            operations.push((s.start_byte, s.end_byte, edit));
        } else {
            anyhow::bail!("AST target not found in source: '{}'. Use `map_overview` first to discover symbol names.", edit.target);
        }
    }

    // Sort descending (Bottom-Up) to preserve byte offsets
    operations.sort_by(|a, b| b.0.cmp(&a.0));

    // 2. Apply edits in-memory
    for (start, end, edit) in operations {
        let prefix = &current_source[..start];
        let suffix = &current_source[end..];
        let replacement = match edit.action.as_str() {
            "delete" => "",
            _        => edit.code.as_str(),
        };
        current_source = format!("{}{}{}", prefix, replacement, suffix);
    }

    // 3. Tree-sitter validation for Rust files (fast, no Wasm needed)
    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext == "rs" {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::language().into()).ok();
        if let Some(tree) = parser.parse(&current_source, None) {
            if tree.root_node().has_error() {
                let ts_errors = collect_ts_errors(tree.root_node(), &current_source);
                eprintln!("[cortex-act] AST validation failed ({} errors). Invoking Auto-Healer...", ts_errors.len());
                current_source = try_auto_heal(file_path, &current_source, &ts_errors, llm_url)?;

                // Final re-check
                if let Some(final_tree) = parser.parse(&current_source, None) {
                    if final_tree.root_node().has_error() {
                        anyhow::bail!("Auto-Healer produced code still containing syntax errors. Edit aborted safely.");
                    }
                }
            }
        }
    }

    // 4. Commit to disk
    std::fs::write(file_path, &current_source).context("Failed to write to file")?;
    Ok(current_source)
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn check_write_permission(path: &Path) -> Result<()> {
    let meta = std::fs::metadata(path)
        .with_context(|| format!("Cannot stat {:?} — file may not exist", path))?;
    if meta.permissions().readonly() {
        anyhow::bail!("Permission denied: {:?} is read-only.", path);
    }
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .with_context(|| format!("Write permission denied on {:?}.", path))?;
    Ok(())
}

fn collect_ts_errors(node: tree_sitter::Node, source: &str) -> Vec<String> {
    let mut errors = Vec::new();
    collect_ts_errors_inner(node, source, &mut errors);
    errors
}

fn collect_ts_errors_inner(node: tree_sitter::Node, source: &str, out: &mut Vec<String>) {
    if node.is_error() || node.is_missing() {
        let row = node.start_position().row + 1;
        let col = node.start_position().column + 1;
        let snippet: String = source
            .get(node.start_byte()..node.end_byte())
            .unwrap_or("<unknown>")
            .chars().take(40).collect();
        if node.is_missing() {
            out.push(format!("Missing '{}' at line {}:{}", node.kind(), row, col));
        } else {
            out.push(format!("Unexpected '{}' at line {}:{}", snippet.trim(), row, col));
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_ts_errors_inner(child, source, out);
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_rs(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(".rs").tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn bottom_up_sort_preserves_byte_offsets() {
        let source = "AAAA BBBB CCCC";
        let mut ops: Vec<(usize, usize, &str)> = vec![
            (0, 4, "X"), (5, 9, "Y"), (10, 14, "Z"),
        ];
        ops.sort_by(|a, b| b.0.cmp(&a.0));
        let mut buf = source.to_string();
        for (start, end, rep) in ops {
            buf = format!("{}{}{}", &buf[..start], rep, &buf[end..]);
        }
        assert_eq!(buf, "X Y Z");
    }

    #[test]
    fn ts_error_collection_on_broken_rust() {
        use tree_sitter::Parser;
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::language().into()).unwrap();
        let broken = "fn broken() { let x = 5;";
        let tree = parser.parse(broken, None).unwrap();
        assert!(tree.root_node().has_error());
        let errors = collect_ts_errors(tree.root_node(), broken);
        assert!(!errors.is_empty());
    }

    #[test]
    fn permission_guard_catches_readonly() {
        use std::os::unix::fs::PermissionsExt;
        let f = temp_rs("fn main() {}");
        let path = f.path();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o444)).unwrap();
        assert!(check_write_permission(path).is_err());
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).ok();
    }

    #[test]
    fn permission_guard_passes_for_writable() {
        let f = temp_rs("fn main() {}");
        assert!(check_write_permission(f.path()).is_ok());
    }
}
