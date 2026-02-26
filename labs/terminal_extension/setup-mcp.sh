#!/bin/bash

echo "🚀 Setting up Terminal Master MCP Server..."

# Install dependencies
echo "📦 Installing dependencies..."
cd mcp-server
npm install

# Build the server
echo "🔨 Building the server..."
npm run build

echo "✅ MCP Server is ready!"
echo ""
echo "📋 Next Steps:"
echo "1. Add the server to your MCP client configuration"
echo "2. Copy this configuration to your claude_desktop_config.json:"
echo ""
cat claude_desktop_config.json
echo ""
echo "3. Restart your MCP client (like Claude Desktop)"
echo "4. Try asking: 'List all active terminals'"
