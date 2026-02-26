#!/bin/bash

echo "🔧 Terminal Master - MCP Integration Setup"
echo "========================================="

set -e

# Build MCP server
echo "� Building MCP server..."
cd mcp-server
npm ci || npm install
npm run build
cd ..

SERVER_PATH="$(pwd)/mcp-server/dist/index.js"
echo "📍 Server: $SERVER_PATH"

# Create minimal MCP configuration
cat > mcp-config.json << EOF
{
  "mcpServers": {
    "terminal-master": {
      "command": "node",
      "args": ["$SERVER_PATH"],
      "env": { "NODE_ENV": "production" }
    }
  }
}
EOF

echo "✅ MCP configuration written to $(pwd)/mcp-config.json"
echo "Next: copy into your MCP client (e.g., Claude Desktop config)."
