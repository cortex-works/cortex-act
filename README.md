# cortex-act 🖐️

> **The AI-Native Code Action Backend** — the "hands" of the Cortex ecosystem.
>
> `cortex-ast` sees. `cortex-act` does.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org)

---

## Overview

`cortex-act` is a pure-Rust **MCP (Model Context Protocol) server** that provides AI coding agents with write, edit, and execute capabilities. It is deliberately scoped to **output-only** operations to enforce a strict separation of concerns:

| Project | Role | Capability |
|---------|------|------------|
| `CortexAST` | 👁️ Eyes | Read-only: code analysis, symbol lookup, semantic navigation |
| **`cortex-act`** | ✋ Hands | Write/execute: file edits, config patching, shell commands |
| `CortexSync` | 🧠 Brain | Global memory: captures intent/decisions, vectorizes memories |

Together, they form the **CortexSync Ecosystem** — a seamless, cross-IDE memory and action layer for AI agents.

> [!IMPORTANT]
> To enable full ecosystem features (like task-end memory capture), ensure `cortex-sync` is running globally and `CortexAST` is installed as your primary MCP server.

---

## Tools

### 1. ✏️ `cortex_act_edit_ast`
Replace or delete a named symbol (function/class/struct) in any source file. Targets by name, not line number. Auto-heals broken AST via local LLM if validation fails. Use `cortexast map_overview` to discover symbol names first.

### 2. ⚙️ `cortex_patch_file`
Surgically patch config (JSON/YAML/TOML via dot-path), markdown docs (section heading), or .env (key). Avoids full-file rewrites. 
- `type=config`: `target='dependencies.serde'`
- `type=docs`: `target='Installation'`
- `type=env`: `target='API_KEY'`

### 3. ⏳ `cortex_act_run_async`
Run a shell command as a background job. Returns immediately with `job_id`. Poll with `cortex_check_job`. Use for long-running builds, scripts, or any command that may exceed MCP timeout.

### 4. 📊 `cortex_check_job`
Poll a background job (from `cortex_act_run_async`). Returns status (`running`/`done`/`failed`), exit code, `duration_secs`, and last 20 lines of output (`log_tail`).

### 5. 🛑 `cortex_kill_job`
Terminate a background job (SIGTERM). No-op if job already finished.

---

## Architecture

```
cortex-act/
├── src/
│   ├── main.rs            # MCP stdio server (JSON-RPC 2.0)
│   └── act/
│       ├── mod.rs
│       ├── editor.rs      # AST Semantic Patcher + Tree-sitter validator
│       ├── auto_healer.rs # LLM-based syntax error repair (10s timeout)
│       ├── config_patcher.rs  # JSON / YAML / TOML dot-path editor
│       ├── docs_patcher.rs    # Markdown section replacer
│       ├── env_patcher.rs     # .env key-value patcher
│       └── job_manager.rs    # Async background job runner + file logging
```

## MCP Configuration

Add to your `mcp_config.json`:

```json
{
  "mcpServers": {
    "cortex-act": {
      "command": "/path/to/cortex-act/target/release/cortex-act",
      "args": []
    }
  }
}
```

## Building

```bash
cargo build --release
# Binary: target/release/cortex-act
```

## Design Principles

1. **Single Responsibility** — Only performs write/execute operations. Never reads or analyzes code.
2. **Two-Phase Commit** — All AST edits go through a virtual dry-run before touching disk.
3. **Auto-Healing** — Syntax errors trigger an LLM repair loop with a strict 10-second timeout.
4. **File-based Job Logs** — Background job output goes to `~/.cortexast/jobs/{job_id}.log` to prevent OOM.
5. **Zero unsafe Rust** — All edits are panic-free and use structured `anyhow::Result` error handling.

## Author

**Thanon Aphithanawat** — [thanon@aphithanawat.me](mailto:thanon@aphithanawat.me)

## License

MIT © 2026 Thanon Aphithanawat
