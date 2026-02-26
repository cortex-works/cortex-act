#!/usr/bin/env node
"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const index_js_1 = require("@modelcontextprotocol/sdk/server/index.js");
const stdio_js_1 = require("@modelcontextprotocol/sdk/server/stdio.js");
const types_js_1 = require("@modelcontextprotocol/sdk/types.js");
const child_process_1 = require("child_process");
const util_1 = require("util");
const execAsync = (0, util_1.promisify)(child_process_1.exec);
class TerminalMasterMCPServer {
    constructor() {
        this.server = new index_js_1.Server({
            name: 'terminal-master-mcp-server',
            version: '1.0.0',
        }, {
            capabilities: {
                tools: {},
            },
        });
        this.setupToolHandlers();
        this.setupErrorHandling();
    }
    setupErrorHandling() {
        this.server.onerror = (error) => {
            console.error('[MCP Error]', error);
        };
        process.on('SIGINT', async () => {
            await this.server.close();
            process.exit(0);
        });
    }
    setupToolHandlers() {
        this.server.setRequestHandler(types_js_1.ListToolsRequestSchema, async () => {
            return {
                tools: [
                    {
                        name: 'list_terminals',
                        description: 'Get all active terminals with their recent output (last 10 lines)',
                        inputSchema: {
                            type: 'object',
                            properties: {},
                            required: []
                        }
                    },
                    {
                        name: 'run_command',
                        description: 'Execute a command in a specific terminal by PID',
                        inputSchema: {
                            type: 'object',
                            properties: {
                                pid: {
                                    type: 'number',
                                    description: 'The process ID of the terminal'
                                },
                                command: {
                                    type: 'string',
                                    description: 'The command to execute'
                                }
                            },
                            required: ['pid', 'command']
                        }
                    },
                    {
                        name: 'run_and_wait',
                        description: 'Execute a command and wait for output (perfect for builds, tests, installs)',
                        inputSchema: {
                            type: 'object',
                            properties: {
                                pid: {
                                    type: 'number',
                                    description: 'The process ID of the terminal'
                                },
                                command: {
                                    type: 'string',
                                    description: 'The command to execute'
                                },
                                waitTime: {
                                    type: 'number',
                                    description: 'How long to wait for output in milliseconds (default: 5000)',
                                    default: 5000
                                }
                            },
                            required: ['pid', 'command']
                        }
                    },
                    {
                        name: 'search_output',
                        description: 'Search for specific keywords in terminal output (great for finding errors)',
                        inputSchema: {
                            type: 'object',
                            properties: {
                                pid: {
                                    type: 'number',
                                    description: 'The process ID of the terminal to search'
                                },
                                keyword: {
                                    type: 'string',
                                    description: 'The keyword to search for (case-insensitive)'
                                }
                            },
                            required: ['pid', 'keyword']
                        }
                    },
                    {
                        name: 'read_terminal_output',
                        description: 'Read specific lines from terminal buffer',
                        inputSchema: {
                            type: 'object',
                            properties: {
                                pid: {
                                    type: 'number',
                                    description: 'The process ID of the terminal'
                                },
                                startLine: {
                                    type: 'number',
                                    description: 'Starting line number (optional)'
                                },
                                endLine: {
                                    type: 'number',
                                    description: 'Ending line number (optional)'
                                }
                            },
                            required: ['pid']
                        }
                    },
                    {
                        name: 'get_extension_status',
                        description: 'Get Terminal Master extension status and overview',
                        inputSchema: {
                            type: 'object',
                            properties: {},
                            required: []
                        }
                    },
                    {
                        name: 'kill_all_terminals',
                        description: 'Close all open terminals (use with caution!)',
                        inputSchema: {
                            type: 'object',
                            properties: {},
                            required: []
                        }
                    },
                    {
                        name: 'generate_test_output',
                        description: 'Generate test output in a terminal for demonstration purposes',
                        inputSchema: {
                            type: 'object',
                            properties: {},
                            required: []
                        }
                    }
                ]
            };
        });
        this.server.setRequestHandler(types_js_1.CallToolRequestSchema, async (request) => {
            const { name, arguments: args } = request.params;
            try {
                switch (name) {
                    case 'list_terminals':
                        return await this.listTerminals();
                    case 'run_command':
                        return await this.runCommand(args.pid, args.command);
                    case 'run_and_wait':
                        return await this.runAndWait(args.pid, args.command, args.waitTime);
                    case 'search_output':
                        return await this.searchOutput(args.pid, args.keyword);
                    case 'read_terminal_output':
                        return await this.readTerminalOutput(args.pid, args.startLine, args.endLine);
                    case 'get_extension_status':
                        return await this.getExtensionStatus();
                    case 'kill_all_terminals':
                        return await this.killAllTerminals();
                    case 'generate_test_output':
                        return await this.generateTestOutput();
                    default:
                        throw new Error(`Unknown tool: ${name}`);
                }
            }
            catch (error) {
                return {
                    content: [
                        {
                            type: 'text',
                            text: `Error: ${error instanceof Error ? error.message : String(error)}`
                        }
                    ],
                    isError: true
                };
            }
        });
    }
    async executeVSCodeCommand(command, args) {
        return new Promise((resolve, reject) => {
            const vsCodeCommand = args
                ? `code --command "${command}" --args '${JSON.stringify(args)}'`
                : `code --command "${command}"`;
            (0, child_process_1.exec)(vsCodeCommand, (error, stdout, stderr) => {
                if (error) {
                    reject(new Error(`VS Code command failed: ${error.message}`));
                    return;
                }
                try {
                    // Try to parse JSON output
                    const result = stdout.trim() ? JSON.parse(stdout) : { success: true };
                    resolve(result);
                }
                catch (parseError) {
                    // If not JSON, return as text
                    resolve({ output: stdout, stderr });
                }
            });
        });
    }
    async listTerminals() {
        try {
            const result = await this.executeVSCodeCommand('terminal-master.list');
            return {
                content: [
                    {
                        type: 'text',
                        text: `Found ${result.length || 0} terminals:\n\n${JSON.stringify(result, null, 2)}`
                    }
                ]
            };
        }
        catch (error) {
            return {
                content: [
                    {
                        type: 'text',
                        text: `Error listing terminals: ${error instanceof Error ? error.message : String(error)}\n\nMake sure VS Code is running and Terminal Master extension is installed.`
                    }
                ],
                isError: true
            };
        }
    }
    async runCommand(pid, command) {
        try {
            const result = await this.executeVSCodeCommand('terminal-master.run', { pid, command });
            return {
                content: [
                    {
                        type: 'text',
                        text: `Command executed successfully:\n\nPID: ${pid}\nCommand: ${command}\nResult: ${JSON.stringify(result, null, 2)}`
                    }
                ]
            };
        }
        catch (error) {
            return {
                content: [
                    {
                        type: 'text',
                        text: `Error running command: ${error instanceof Error ? error.message : String(error)}`
                    }
                ],
                isError: true
            };
        }
    }
    async runAndWait(pid, command, waitTime = 5000) {
        try {
            const result = await this.executeVSCodeCommand('terminal-master.runAndWait', {
                pid,
                command,
                waitTime
            });
            return {
                content: [
                    {
                        type: 'text',
                        text: `Command executed and waited for output:\n\n` +
                            `Command: ${command}\n` +
                            `PID: ${pid}\n` +
                            `Wait Time: ${waitTime}ms\n\n` +
                            `Captured Output:\n${result.capturedOutput?.join('\n') || 'No output captured'}\n\n` +
                            `Full Result: ${JSON.stringify(result, null, 2)}`
                    }
                ]
            };
        }
        catch (error) {
            return {
                content: [
                    {
                        type: 'text',
                        text: `Error running command with wait: ${error instanceof Error ? error.message : String(error)}`
                    }
                ],
                isError: true
            };
        }
    }
    async searchOutput(pid, keyword) {
        try {
            const result = await this.executeVSCodeCommand('terminal-master.search', { pid, keyword });
            return {
                content: [
                    {
                        type: 'text',
                        text: `Search results for "${keyword}" in terminal ${pid}:\n\n` +
                            `Found ${result.matchingLines?.length || 0} matching lines:\n\n` +
                            `${result.matchingLines?.join('\n') || 'No matches found'}\n\n` +
                            `Full Result: ${JSON.stringify(result, null, 2)}`
                    }
                ]
            };
        }
        catch (error) {
            return {
                content: [
                    {
                        type: 'text',
                        text: `Error searching output: ${error instanceof Error ? error.message : String(error)}`
                    }
                ],
                isError: true
            };
        }
    }
    async readTerminalOutput(pid, startLine, endLine) {
        try {
            const args = { pid };
            if (startLine !== undefined)
                args.startLine = startLine;
            if (endLine !== undefined)
                args.endLine = endLine;
            const result = await this.executeVSCodeCommand('terminal-master.read', args);
            return {
                content: [
                    {
                        type: 'text',
                        text: `Terminal output for PID ${pid}:\n\n` +
                            `Lines: ${startLine || 0} to ${endLine || 'end'}\n\n` +
                            `${result.lines?.join('\n') || 'No output available'}\n\n` +
                            `Full Result: ${JSON.stringify(result, null, 2)}`
                    }
                ]
            };
        }
        catch (error) {
            return {
                content: [
                    {
                        type: 'text',
                        text: `Error reading terminal output: ${error instanceof Error ? error.message : String(error)}`
                    }
                ],
                isError: true
            };
        }
    }
    async getExtensionStatus() {
        try {
            const result = await this.executeVSCodeCommand('terminal-master.getStatus');
            return {
                content: [
                    {
                        type: 'text',
                        text: `Terminal Master Extension Status:\n\n` +
                            `Active: ${result.extensionActive ? '✅ Yes' : '❌ No'}\n` +
                            `Total Terminals: ${result.totalTerminals || 0}\n` +
                            `Buffered Terminals: ${result.bufferedTerminals || 0}\n` +
                            `Active Executions: ${result.activeExecutions || 0}\n` +
                            `Timestamp: ${result.timestamp}\n\n` +
                            `Terminals:\n${result.terminals?.map((t) => `- ${t.name} (PID: ${t.pid}) - Buffer: ${t.bufferSize} lines`).join('\n') || 'No terminals'}\n\n` +
                            `Full Status: ${JSON.stringify(result, null, 2)}`
                    }
                ]
            };
        }
        catch (error) {
            return {
                content: [
                    {
                        type: 'text',
                        text: `Error getting extension status: ${error instanceof Error ? error.message : String(error)}`
                    }
                ],
                isError: true
            };
        }
    }
    async killAllTerminals() {
        try {
            const result = await this.executeVSCodeCommand('terminal-master.killAll');
            return {
                content: [
                    {
                        type: 'text',
                        text: `All terminals killed:\n\n` +
                            `Killed ${result.killedCount || 0} terminals\n` +
                            `Result: ${JSON.stringify(result, null, 2)}`
                    }
                ]
            };
        }
        catch (error) {
            return {
                content: [
                    {
                        type: 'text',
                        text: `Error killing terminals: ${error instanceof Error ? error.message : String(error)}`
                    }
                ],
                isError: true
            };
        }
    }
    async generateTestOutput() {
        try {
            const result = await this.executeVSCodeCommand('terminal-master.generate-test-output');
            return {
                content: [
                    {
                        type: 'text',
                        text: `Test output generation initiated:\n\n` +
                            `Terminal: ${result.terminal || 'Unknown'}\n` +
                            `PID: ${result.pid || 'Unknown'}\n` +
                            `Message: ${result.message || 'Generated test output'}\n` +
                            `Note: ${result.note || 'Wait a moment then use list_terminals to see captured output'}\n\n` +
                            `Full Result: ${JSON.stringify(result, null, 2)}`
                    }
                ]
            };
        }
        catch (error) {
            return {
                content: [
                    {
                        type: 'text',
                        text: `Error generating test output: ${error instanceof Error ? error.message : String(error)}`
                    }
                ],
                isError: true
            };
        }
    }
    async run() {
        const transport = new stdio_js_1.StdioServerTransport();
        await this.server.connect(transport);
        console.error('Terminal Master MCP Server running on stdio');
        console.error('Available tools: list_terminals, run_command, run_and_wait, search_output, read_terminal_output, get_extension_status, kill_all_terminals, generate_test_output');
    }
}
const server = new TerminalMasterMCPServer();
server.run().catch(console.error);
//# sourceMappingURL=index.js.map