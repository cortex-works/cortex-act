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
| `cortex-ast` | 👁️ Eyes | Read-only: code analysis, symbol lookup, semantic navigation |
| **`cortex-act`** | ✋ Hands | Write/execute: file edits, config patching, shell commands |

---

## Tools

### 🔧 `cortex_act_edit_ast`
**AST Semantic Patcher** — Apply surgical code edits to source files using Tree-sitter byte-accurate targeting.
- Two-Phase Commit: dry-run → validate → commit
- Auto-Healer: sends broken code to a local LLM (LM Studio/Ollama) on syntax error
- Never uses line numbers — targets semantic nodes by name
- Supported: Rust (Tree-sitter), all other languages (regex fallback)

**Parameters:** `file`, `edits[]` (`target`, `action`, `code`), `llm_url?`

---

### 🔧 `cortex_patch_file`
**Unified File Patcher** — Surgically modify config, docs, or env files without rewriting the whole file.

| `type` | Target format | `target` field |
|--------|-------------|----------------|
| `config` | JSON / YAML / TOML | Dot-path e.g. `"dependencies.serde"` |
| `docs` | Markdown | Heading text e.g. `"Installation"` |
| `env` | `.env` key-value | Key name e.g. `"OPENAI_API_KEY"` |

**Parameters:** `file`, `type`, `action` (set\|delete), `target`, `value?`, `heading_level?`

---

### ⏳ `cortex_act_run_async`
**Async Job Runner** — Spawn shell commands as background jobs. Returns immediately with a `job_id`.

**Parameters:** `command`, `cwd?`, `timeout_secs?`

---

### 📊 `cortex_check_job`
**Job Status** — Poll a background job. Returns status, PID, exit code, duration, and last 20 lines of log.

**Parameters:** `job_id`

---

### 🛑 `cortex_kill_job`
**Kill Job** — Send SIGTERM to a running job and mark it as failed.

**Parameters:** `job_id`

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
