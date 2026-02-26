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
                        "description": "Replace or delete a named symbol (function/class/struct) in any source file. Targets by name, not line number. Auto-heals broken AST via local LLM if validation fails. Use cortexast map_overview to discover symbol names first.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file": { "type": "string", "description": "Abs path to source file." },
                                "edits": {
                                    "type": "array",
                                    "description": "Edits to apply (bottom-up order enforced automatically).",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "target": { "type": "string", "description": "Symbol name or 'kind:name' (e.g. 'login' or 'function:login')." },
                                            "action": { "type": "string", "enum": ["replace", "delete"], "description": "replace: swap entire symbol body. delete: remove symbol." },
                                            "code":   { "type": "string", "description": "Full replacement source (required for replace)." }
                                        },
                                        "required": ["target", "action"]
                                    }
                                },
                                "llm_url": { "type": "string", "description": "Auto-Healer LLM endpoint override. Default: http://127.0.0.1:1234/v1/chat/completions." }
                            },
                            "required": ["file", "edits"]
                        }
                    },
                    // ── Unified File Patcher (Config / Docs / Env) ──────────
                    {
                        "name": "cortex_patch_file",
                        "description": "Surgically patch config (JSON/YAML/TOML via dot-path), markdown docs (section heading), or .env (key). Avoids full-file rewrites. type=config: target='dependencies.serde'. type=docs: target='Installation'. type=env: target='API_KEY'.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file":          { "type": "string", "description": "Abs path to target file." },
                                "type":          { "type": "string", "enum": ["config", "docs", "env"], "description": "File type to patch." },
                                "action":        { "type": "string", "enum": ["set", "delete"], "description": "set: upsert value. delete: remove key/section." },
                                "target":        { "type": "string", "description": "Dot-path for config, heading text for docs, key name for env." },
                                "value":         { "description": "New value (required for set)." },
                                "heading_level": { "type": "integer", "description": "Heading level for docs (1-4). Default 2 (##).", "default": 2 }
                            },
                            "required": ["file", "type", "action", "target"]
                        }
                    },
                    // ── Async Job Runner ─────────────────────────────────────
                    {
                        "name": "cortex_act_run_async",
                        "description": "Run a shell command as a background job. Returns immediately with job_id. Poll with cortex_check_job. Use for long-running builds, scripts, or any command that may exceed MCP timeout.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "command":      { "type": "string",  "description": "Shell command to execute." },
                                "cwd":          { "type": "string",  "description": "Working directory. Default: cwd." },
                                "timeout_secs": { "type": "integer", "description": "Hard timeout seconds. Default 300.", "default": 300 }
                            },
                            "required": ["command"]
                        }
                    },
                    {
                        "name": "cortex_check_job",
                        "description": "Poll a background job (from cortex_act_run_async). Returns status (running/done/failed), exit code, duration_secs, and last 20 lines of output (log_tail).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "job_id": { "type": "string", "description": "Job ID from cortex_act_run_async." }
                            },
                            "required": ["job_id"]
                        }
                    },
                    {
                        "name": "cortex_kill_job",
                        "description": "Terminate a background job (SIGTERM). No-op if job already finished.",
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
