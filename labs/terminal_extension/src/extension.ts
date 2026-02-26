import * as vscode from 'vscode';

interface TerminalBuffer {
    name: string;
    pid: number;
    lines: string[];
}

export function activate(context: vscode.ExtensionContext) {
    console.log('🚀 Terminal Master extension activated');
    console.log('Extension context:', context.extensionPath);
    
    // Minimal activation: no popup
    
    // Map to store terminal buffers using processId as key
    const terminalBuffers = new Map<number, TerminalBuffer>();
    // Map to track active executions for reading data streams
    const activeExecutions = new Map<vscode.TerminalShellExecution, AsyncIterator<string>>();
    
    // Helper function to get terminal PID
    async function getTerminalPid(terminal: vscode.Terminal): Promise<number | undefined> {
        return await terminal.processId;
    }
    
    // Initialize buffer for a terminal
    async function initializeBuffer(terminal: vscode.Terminal) {
        const pid = await getTerminalPid(terminal);
        if (pid && !terminalBuffers.has(pid)) {
            terminalBuffers.set(pid, {
                name: terminal.name,
                pid: pid,
                lines: []
            });
        }
        return pid;
    }
    
    // Add data to terminal buffer
    function addDataToBuffer(pid: number, data: string) {
        const buffer = terminalBuffers.get(pid);
        if (!buffer) return;
        
        // Split the data into lines and append to buffer
        const newLines = data.split('\n');
        
        // Handle the case where the first line should be appended to the last line
        if (buffer.lines.length > 0 && newLines.length > 0) {
            const lastLine = buffer.lines[buffer.lines.length - 1];
            buffer.lines[buffer.lines.length - 1] = lastLine + newLines[0];
            newLines.shift(); // Remove the first line as it's been merged
        }
        
        // Add remaining lines
        buffer.lines.push(...newLines);
        
        // Keep only the last 1000 lines to prevent memory issues
        if (buffer.lines.length > 1000) {
            buffer.lines = buffer.lines.slice(-1000);
        }
    }
    
    // Listen for shell integration changes to track new terminals
    const shellIntegrationListener = vscode.window.onDidChangeTerminalShellIntegration(async (event) => {
        console.log('🔗 Shell integration changed for terminal:', event.terminal.name);
        await initializeBuffer(event.terminal);
    });
    
    // Listen for shell execution starts to capture command output
    const executionStartListener = vscode.window.onDidStartTerminalShellExecution(async (event) => {
        const pid = await initializeBuffer(event.terminal);
        if (!pid) return;
        
        console.log(`🚀 Command started in terminal ${event.terminal.name} (PID: ${pid})`);
        
        // Start reading the execution output
        try {
            const stream = event.execution.read();
            const iterator = stream[Symbol.asyncIterator]();
            activeExecutions.set(event.execution, iterator);
            
            // Read data in background
            (async () => {
                try {
                    let result = await iterator.next();
                    while (!result.done) {
                        console.log(`📝 Adding data to buffer for PID ${pid}:`, result.value.substring(0, 100) + '...');
                        addDataToBuffer(pid, result.value);
                        result = await iterator.next();
                    }
                } catch (error) {
                    console.error('Error reading terminal execution:', error);
                } finally {
                    activeExecutions.delete(event.execution);
                }
            })();
        } catch (error) {
            console.error('Error setting up terminal execution reader:', error);
        }
    });
    
    // Clean up when execution ends
    const executionEndListener = vscode.window.onDidEndTerminalShellExecution((event) => {
        console.log(`✅ Command ended in terminal ${event.terminal.name}`);
        const iterator = activeExecutions.get(event.execution);
        if (iterator) {
            activeExecutions.delete(event.execution);
        }
    });
    
    // Clean up buffers when terminals are closed
    const terminalCloseListener = vscode.window.onDidCloseTerminal(async (terminal) => {
        const pid = await getTerminalPid(terminal);
        if (pid) {
            console.log(`🗑️ Cleaning up buffer for closed terminal PID: ${pid}`);
            terminalBuffers.delete(pid);
        }
    });
    
    // Initialize buffers for existing terminals
    (async () => {
        for (const terminal of vscode.window.terminals) {
            await initializeBuffer(terminal);
            console.log(`📋 Initialized buffer for existing terminal: ${terminal.name}`);
        }
    })();
    
    // Minimal command set only: list, run, read, runAndWait
    
    // Command: terminal-master.list
    const listCommand = vscode.commands.registerCommand('terminal-master.list', async () => {
        console.log('📋 List command executed');
        const activeTerminals = vscode.window.terminals;
        console.log(`Found ${activeTerminals.length} terminals`);
        
        const result: Array<{name: string; pid: number; latestLines: string[]}> = [];
        
        // For each terminal, we'll send a command to get recent history
        for (let i = 0; i < activeTerminals.length; i++) {
            const terminal = activeTerminals[i];
            const pid = await getTerminalPid(terminal);
            console.log(`Terminal "${terminal.name}" has PID: ${pid}`);
            
            if (pid) {
                // Initialize buffer if it doesn't exist
                await initializeBuffer(terminal);
                
                let latestLines: string[] = [];
                
                // Check if we have buffered data from previous shell executions
                if (terminalBuffers.has(pid)) {
                    const buffer = terminalBuffers.get(pid)!;
                    if (buffer.lines.length > 0) {
                        latestLines = buffer.lines.slice(-10).filter(line => line.trim() !== '');
                        console.log(`Found ${latestLines.length} buffered lines for PID ${pid}`);
                    }
                }
                
                // If no buffered data, provide terminal info
                if (latestLines.length === 0) {
                    latestLines = [
                        `Terminal: ${terminal.name}`,
                        `PID: ${pid}`,
                        `Status: Active`,
                        `Note: Run commands in this terminal to see output here`,
                        `Try: echo "Hello Terminal Master"`
                    ];
                }
                
                result.push({
                    name: terminal.name,
                    pid: pid,
                    latestLines: latestLines.slice(-10) // Max 10 lines
                });
            } else {
                result.push({
                    name: terminal.name,
                    pid: 0,
                    latestLines: ['Unable to get terminal PID']
                });
            }
        }
        
        console.log('📋 Final result:', JSON.stringify(result, null, 2));
        vscode.window.showInformationMessage(`Found ${result.length} terminals - check console for JSON output`);
        return result;
    });
    
    // Command: terminal-master.run
    const runCommand = vscode.commands.registerCommand('terminal-master.run', async (input?: any) => {
        let params = input;
        
        if (!params) {
            const inputStr = await vscode.window.showInputBox({
                prompt: 'Enter JSON with pid and command',
                placeHolder: '{"pid": 12345, "command": "echo hello"}'
            });
            
            if (!inputStr) {
                return;
            }
            
            try {
                params = JSON.parse(inputStr);
            } catch (error) {
                vscode.window.showErrorMessage('Invalid JSON format');
                return;
            }
        }
        
        if (!params.pid || !params.command) {
            const error = { error: 'Missing required parameters: pid and command' };
            console.log(JSON.stringify(error, null, 2));
            return error;
        }
        
        // Find terminal with matching PID
        let targetTerminal: vscode.Terminal | undefined;
        for (const terminal of vscode.window.terminals) {
            const pid = await getTerminalPid(terminal);
            if (pid === params.pid) {
                targetTerminal = terminal;
                break;
            }
        }
        
        if (!targetTerminal) {
            const error = { error: `Terminal with pid ${params.pid} not found` };
            console.log(JSON.stringify(error, null, 2));
            return error;
        }
        
        targetTerminal.sendText(params.command);
        const result = { success: `Command "${params.command}" sent to terminal ${params.pid}` };
        console.log(JSON.stringify(result, null, 2));
        return result;
    });
    
    // Command: terminal-master.read
    const readCommand = vscode.commands.registerCommand('terminal-master.read', async (input?: any) => {
        let params = input;
        
        if (!params) {
            const inputStr = await vscode.window.showInputBox({
                prompt: 'Enter JSON with pid and optional startLine/endLine',
                placeHolder: '{"pid": 12345, "startLine": 0, "endLine": 10}'
            });
            
            if (!inputStr) {
                return;
            }
            
            try {
                params = JSON.parse(inputStr);
            } catch (error) {
                vscode.window.showErrorMessage('Invalid JSON format');
                return;
            }
        }
        
        if (!params.pid) {
            const error = { error: 'Missing required parameter: pid' };
            console.log(JSON.stringify(error, null, 2));
            return error;
        }
        
        const buffer = terminalBuffers.get(params.pid);
        
        if (!buffer) {
            const error = { error: `Terminal buffer for pid ${params.pid} not found` };
            console.log(JSON.stringify(error, null, 2));
            return error;
        }
        
        let lines = buffer.lines;
        
        if (typeof params.startLine === 'number' && typeof params.endLine === 'number') {
            lines = lines.slice(params.startLine, params.endLine + 1);
        } else if (typeof params.startLine === 'number') {
            lines = lines.slice(params.startLine);
        } else if (typeof params.endLine === 'number') {
            lines = lines.slice(0, params.endLine + 1);
        }
        
        const result = {
            pid: params.pid,
            lines: lines
        };
        
        console.log(JSON.stringify(result, null, 2));
        return result;
    });
    
    
    // Command: terminal-master.runAndWait (AI-friendly command execution)
    const runAndWaitCommand = vscode.commands.registerCommand('terminal-master.runAndWait', async (input?: any) => {
        let params = input;
        
        if (!params) {
            const inputStr = await vscode.window.showInputBox({
                prompt: 'Enter JSON with pid, command, and optional waitTime (ms)',
                placeHolder: '{"pid": 12345, "command": "echo hello", "waitTime": 2000}'
            });
            
            if (!inputStr) {
                return;
            }
            
            try {
                params = JSON.parse(inputStr);
            } catch (error) {
                const errorResult = { error: 'Invalid JSON format' };
                console.log(JSON.stringify(errorResult, null, 2));
                return errorResult;
            }
        }
        
        if (!params.pid || !params.command) {
            const error = { error: 'Missing required parameters: pid and command' };
            console.log(JSON.stringify(error, null, 2));
            return error;
        }
        
        // Find terminal with matching PID
        let targetTerminal: vscode.Terminal | undefined;
        for (const terminal of vscode.window.terminals) {
            const pid = await getTerminalPid(terminal);
            if (pid === params.pid) {
                targetTerminal = terminal;
                break;
            }
        }
        
        if (!targetTerminal) {
            const error = { error: `Terminal with pid ${params.pid} not found` };
            console.log(JSON.stringify(error, null, 2));
            return error;
        }
        
        // Send the command
        targetTerminal.sendText(params.command);
        
        // Wait for output (default 3 seconds)
        const waitTime = params.waitTime || 3000;
        await new Promise(resolve => setTimeout(resolve, waitTime));
        
        // Get the captured output
        const buffer = terminalBuffers.get(params.pid);
        const capturedOutput = buffer ? buffer.lines.slice(-10) : [];
        
        const result = {
            success: true,
            command: params.command,
            pid: params.pid,
            terminal: targetTerminal.name,
            waitTime: waitTime,
            capturedOutput: capturedOutput,
            timestamp: new Date().toISOString()
        };
        
        console.log('⏳ Run and Wait result:', JSON.stringify(result, null, 2));
        return result;
    });
    
    // Register all disposables
    context.subscriptions.push(
        shellIntegrationListener,
        executionStartListener,
        executionEndListener,
        terminalCloseListener,
        listCommand,
        runCommand,
        readCommand,
        runAndWaitCommand
    );
}

export function deactivate() {
    console.log('Terminal Master extension deactivated');
}
