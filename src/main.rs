use std::io::{BufRead, Write};
use serde_json::{json, Value};
use anyhow::Result;

mod act;

fn main() -> Result<()> {
    let mut server = McpServer::default();
    let stdin  = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }

        let msg: Value = match serde_json::from_str(&line) {
            Ok(v)  => v,
            Err(_) => continue,
        };

        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id     = msg.get("id").cloned().unwrap_or(Value::Null);
        let params = msg.get("params").cloned().unwrap_or(json!({}));

        let response = match method {
            "initialize"         => server.initialize(id, &params),
            "tools/list"         => server.tools_list(id),
            "tools/call"         => server.tool_call(id, &params),
            "notifications/initialized" | "notifications/cancelled" => continue,
            _                    => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {}", method) }
            }),
        };

        let mut s = serde_json::to_string(&response)?;
        s.push('\n');
        out.write_all(s.as_bytes())?;
        out.flush()?;
    }
    Ok(())
}

// ─── Server ───────────────────────────────────────────────────────────────────

#[derive(Default)]
struct McpServer;

impl McpServer {
    fn initialize(&self, id: Value, _params: &Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name":    "cortex-act",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        })
    }

    fn tools_list(&self, id: Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
                    // ── AST Semantic Patcher ──────────────────────────────────
                    {
                        "name": "cortex_act_edit_ast",
                        "description": "🔧 AST SEMANTIC PATCHER — Apply surgical code edits to a source file using Tree-sitter byte-accurate targeting. Edits are applied via Two-Phase Commit (dry-run → validate → commit). If validation detects ERROR nodes, the Auto-Healer automatically sends the broken block to a local LLM for repair within a strict 10-second timeout before safe commit. NEVER uses line numbers — targets semantic nodes by name.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file": { "type": "string", "description": "Absolute path to the file to edit." },
                                "edits": {
                                    "type": "array",
                                    "description": "List of semantic edits to apply (bottom-up patching applied automatically).",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "target": { "type": "string", "description": "Semantic target: 'kind:name' or just 'name'. E.g. 'function:login' or 'login'." },
                                            "action": { "type": "string", "enum": ["replace", "delete"], "description": "Edit action to apply." },
                                            "code":   { "type": "string", "description": "Replacement source code (used for 'replace' action)." }
                                        },
                                        "required": ["target", "action"]
                                    }
                                },
                                "llm_url": { "type": "string", "description": "Optional override URL for the Auto-Healer LLM endpoint. Defaults to http://127.0.0.1:1234/v1/chat/completions." }
                            },
                            "required": ["file", "edits"]
                        }
                    },
                    // ── Unified File Patcher (Config / Docs / Env) ──────────
                    {
                        "name": "cortex_patch_file",
                        "description": "🔧 UNIFIED PATCHER — Surgically modify Config (JSON/YAML/TOML via dot-path), Docs (Markdown section heading), or Env (.env keys). Avoids full-file rewrites and saves tokens.\n• type='config' → target is dot-path e.g. 'dependencies.serde'\n• type='docs'   → target is heading text e.g. 'Installation'\n• type='env'    → target is key name e.g. 'OPENAI_API_KEY'",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file":          { "type": "string", "description": "Absolute path to the target file." },
                                "type":          { "type": "string", "enum": ["config", "docs", "env"], "description": "Type of patching to apply." },
                                "action":        { "type": "string", "enum": ["set", "delete"], "description": "Patch action." },
                                "target":        { "type": "string", "description": "Dot-path (config), Section heading (docs), or Key name (env)." },
                                "value":         { "description": "New value/content to set. Required for 'set' action." },
                                "heading_level": { "type": "integer", "description": "Heading level (1-4) for 'docs' only. Defaults to 2 (##).", "default": 2 }
                            },
                            "required": ["file", "type", "action", "target"]
                        }
                    },
                    // ── Async Job Runner ─────────────────────────────────────
                    {
                        "name": "cortex_act_run_async",
                        "description": "⏳ ASYNC JOB RUNNER — Spawn a terminal command or shell script as a background job. Returns immediately with a job_id to avoid MCP timeout. Use cortex_check_job to poll for results.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "command":      { "type": "string",  "description": "Shell command to run in the background." },
                                "cwd":          { "type": "string",  "description": "Optional working directory for the command." },
                                "timeout_secs": { "type": "integer", "description": "Optional hard timeout in seconds. Defaults to 300.", "default": 300 }
                            },
                            "required": ["command"]
                        }
                    },
                    {
                        "name": "cortex_check_job",
                        "description": "📊 JOB STATUS — Poll a background job started by cortex_act_run_async. Returns status (running/done/failed), PID, exit code, duration_secs, and the last 20 lines of the log file (log_tail).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "job_id": { "type": "string", "description": "Job ID returned by cortex_act_run_async." }
                            },
                            "required": ["job_id"]
                        }
                    },
                    {
                        "name": "cortex_kill_job",
                        "description": "🛑 KILL JOB — Terminate a running background job. Sends SIGTERM to the process and marks it as failed. Safe to call on already-finished jobs (no-op).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "job_id": { "type": "string", "description": "Job ID to terminate." }
                            },
                            "required": ["job_id"]
                        }
                    }
                ]
            }
        })
    }

    fn tool_call(&mut self, id: Value, params: &Value) -> Value {
        let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        let ok  = |text: String| json!({ "jsonrpc": "2.0", "id": id, "result": { "content": [{"type":"text","text": text}], "isError": false } });
        let err = |msg: String|  json!({ "jsonrpc": "2.0", "id": id, "result": { "content": [{"type":"text","text": msg}],  "isError": true  } });

        match name {
            "cortex_act_edit_ast" => {
                let file_str = match args.get("file").and_then(|v| v.as_str()) {
                    Some(s) => s, None => return err("'file' required".to_string()),
                };
                let edits_val = match args.get("edits").and_then(|v| v.as_array()) {
                    Some(a) => a.clone(), None => return err("'edits' array required".to_string()),
                };
                let llm_url = args.get("llm_url").and_then(|v| v.as_str()).map(|s| s.to_string());
                let file_path = std::path::Path::new(file_str);

                let mut edits = Vec::new();
                for item in &edits_val {
                    let target = item.get("target").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let action = item.get("action").and_then(|v| v.as_str()).unwrap_or("replace").to_string();
                    let code   = item.get("code").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if target.is_empty() { return err("Each edit must have a 'target'".to_string()); }
                    edits.push(crate::act::editor::AstEdit { target, action, code });
                }

                match crate::act::editor::apply_ast_edits(file_path, edits, llm_url.as_deref()) {
                    Ok(result) => {
                        let preview: String = result.chars().take(500).collect();
                        ok(serde_json::to_string(&json!({
                            "status": "ok",
                            "message": format!("Applied {} edit(s) to {}", edits_val.len(), file_str),
                            "preview": preview
                        })).unwrap_or_default())
                    }
                    Err(e) => err(format!("cortex_act_edit_ast failed: {}", e)),
                }
            }

            "cortex_patch_file" => {
                let file_str = match args.get("file").and_then(|v| v.as_str()) {
                    Some(s) => s, None => return err("'file' required".to_string()),
                };
                let patch_type = match args.get("type").and_then(|v| v.as_str()) {
                    Some(s) => s, None => return err("'type' required (config|docs|env)".to_string()),
                };
                let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("set");
                let target = match args.get("target").and_then(|v| v.as_str()) {
                    Some(s) => s, None => return err("'target' required".to_string()),
                };

                match patch_type {
                    "env" => {
                        let value = args.get("value").and_then(|v| v.as_str());
                        match crate::act::env_patcher::patch_env(file_str, action, target, value) {
                            Ok(msg) => ok(msg),
                            Err(e)  => err(format!("cortex_patch_file(env) failed: {}", e)),
                        }
                    }
                    "config" => {
                        let value = args.get("value").cloned();
                        match crate::act::config_patcher::patch_config(file_str, action, target, value.as_ref()) {
                            Ok(msg) => ok(msg),
                            Err(e)  => err(format!("cortex_patch_file(config) failed: {}", e)),
                        }
                    }
                    "docs" => {
                        let content = if action == "delete" {
                            ""
                        } else {
                            match args.get("value").and_then(|v| v.as_str()) {
                                Some(s) => s, None => return err("'value' string required for docs 'set' action".to_string()),
                            }
                        };
                        let level = args.get("heading_level").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
                        match crate::act::docs_patcher::patch_docs(file_str, target, content, level) {
                            Ok(msg) => ok(msg),
                            Err(e)  => err(format!("cortex_patch_file(docs) failed: {}", e)),
                        }
                    }
                    other => err(format!("Unknown patch type: '{}'. Use: config | docs | env", other)),
                }
            }

            "cortex_act_run_async" => {
                let command = match args.get("command").and_then(|v| v.as_str()) {
                    Some(s) => s.to_string(), None => return err("'command' required".to_string()),
                };
                let cwd         = args.get("cwd").and_then(|v| v.as_str()).map(|s| s.to_string());
                let timeout_secs = args.get("timeout_secs").and_then(|v| v.as_u64()).unwrap_or(300);
                match crate::act::job_manager::spawn_job(command, cwd, timeout_secs) {
                    Ok(r)  => ok(serde_json::to_string(&r).unwrap_or_default()),
                    Err(e) => err(format!("cortex_act_run_async failed: {}", e)),
                }
            }

            "cortex_check_job" => {
                let job_id = match args.get("job_id").and_then(|v| v.as_str()) {
                    Some(s) => s, None => return err("'job_id' required".to_string()),
                };
                match crate::act::job_manager::check_job(job_id) {
                    Ok(r)  => ok(serde_json::to_string(&r).unwrap_or_default()),
                    Err(e) => err(format!("cortex_check_job failed: {}", e)),
                }
            }

            "cortex_kill_job" => {
                let job_id = match args.get("job_id").and_then(|v| v.as_str()) {
                    Some(s) => s, None => return err("'job_id' required".to_string()),
                };
                match crate::act::job_manager::kill_job(job_id) {
                    Ok(msg) => ok(msg),
                    Err(e)  => err(format!("cortex_kill_job failed: {}", e)),
                }
            }

            other => err(format!("Unknown tool: '{}'. Available tools: cortex_act_edit_ast, cortex_patch_file, cortex_act_run_async, cortex_check_job, cortex_kill_job", other)),
        }
    }
}
