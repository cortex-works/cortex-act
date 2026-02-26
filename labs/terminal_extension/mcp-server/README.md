Terminal Master MCP Server

Build
- npm ci
- npm run build

Run
- node dist/index.js

Tools
- list_terminals
- run_command
- read_terminal_output
- run_and_wait# Terminal Master MCP Server

An **easy-to-use MCP (Model Context Protocol) server** that provides AI agents with simple tools to control the Terminal Master VS Code extension.

## 🚀 Quick Start

### 1. Install Dependencies
```bash
cd mcp-server
npm install
```

### 2. Build the Server
```bash
npm run build
```

### 3. Configure in Claude Desktop (or other MCP client)

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "terminal-master": {
      "command": "node",
      "args": ["/path/to/your/terminal_extension/mcp-server/dist/index.js"]
    }
  }
}
```

## 🛠️ Available Tools for AI Agents

### **`list_terminals`**
Get all active terminals with recent output
```
No parameters needed
```

### **`run_command`**
Execute a command in a specific terminal
```
- pid: Terminal process ID
- command: Command to execute
```

### **`run_and_wait`**
Execute command and wait for output (perfect for builds/tests)
```
- pid: Terminal process ID  
- command: Command to execute
- waitTime: How long to wait (default: 5000ms)
```

### **`search_output`**
Find specific keywords in terminal output
```
- pid: Terminal process ID
- keyword: Text to search for
```

### **`read_terminal_output`**
Read specific lines from terminal buffer
```
- pid: Terminal process ID
- startLine: Starting line (optional)
- endLine: Ending line (optional)
```

### **`get_extension_status`**
Get Terminal Master extension overview
```
No parameters needed
```

### **`kill_all_terminals`**
Close all terminals (use carefully!)
```
No parameters needed
```

### **`generate_test_output`**
Create sample output for testing
```
No parameters needed
```

## 💬 Example AI Agent Conversations

**AI Agent:** "List all active terminals"
- Uses: `list_terminals`
- Gets: JSON with all terminals and their recent output

**AI Agent:** "Run npm test in terminal 12345 and show me the results"
- Uses: `run_and_wait` with pid=12345, command="npm test"
- Gets: Test results after waiting for completion

**AI Agent:** "Search for any errors in terminal output"
- Uses: `search_output` with keyword="error"
- Gets: All lines containing "error"

**AI Agent:** "What's the status of the Terminal Master extension?"
- Uses: `get_extension_status`
- Gets: Overview of extension state and terminals

## 🔧 How It Works

1. **AI Agent calls MCP tool** (e.g., "list_terminals")
2. **MCP Server translates** to VS Code command
3. **Terminal Master Extension** executes and returns JSON
4. **MCP Server formats** response for AI agent
5. **AI Agent receives** clean, readable results

## ✅ Benefits for AI Agents

- **No coding required** - Simple tool calls
- **Natural language** - Tools have descriptive names
- **Rich responses** - Formatted, human-readable output
- **Error handling** - Clear error messages
- **Type safety** - Structured inputs and outputs

## 📋 Prerequisites

1. **VS Code running** with Terminal Master extension installed
2. **Node.js 18+** for the MCP server
3. **MCP Client** (like Claude Desktop) configured

## 🚀 Development

```bash
# Watch mode for development
npm run dev

# Build for production
npm run build

# Start the server
npm start
```

This MCP server makes Terminal Master extension **incredibly easy** for AI agents to use - no complex coding, just simple, descriptive tool calls!
