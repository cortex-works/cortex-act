# Terminal Master (Minimal)

Minimal VS Code extension + MCP server to control existing terminals safely.

What you get:
- terminal-master.list — list terminals with PID and recent lines
- terminal-master.run — send a command to a terminal by PID
- terminal-master.read — read buffered output lines
- terminal-master.runAndWait — send a command and wait, then return last lines

MCP server exposes matching tools: list_terminals, run_command, read_terminal_output, run_and_wait.

## Develop
- Build extension: npm run compile (outputs to out/)
- Build MCP server: (cd mcp-server && npm run build)

## Install/Run
- VSIX: use VS Code to install from out or package via vsce if needed.
- MCP: point your client to mcp-server/dist/index.js per mcp-server/claude_desktop_config.json.

## Notes
- The previous clean_build folder and large root MCP scripts are legacy. Prefer src/ and mcp-server/ only.
