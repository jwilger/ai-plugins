// The subset of pi's ExtensionAPI that this adapter uses. Declared locally so
// the package has no build-time dependency on pi; the shapes are structurally
// compatible with `@earendil-works/pi-coding-agent`'s exported types.

export type NotifyLevel = "info" | "warning" | "error";

export interface PiContext {
  cwd: string;
  ui: { notify(message: string, type?: NotifyLevel): void };
}

export interface ExecResult {
  stdout: string;
  stderr: string;
  code: number;
  killed: boolean;
}

export interface ExecOptions {
  cwd?: string;
  timeout?: number;
}

export type McpExposure = "codemode" | "deferred" | "direct" | "hidden";

export interface McpStdioServerConfig {
  type?: "stdio";
  command: string;
  args?: string[];
  exposure?: McpExposure;
  description?: string;
}

export interface PiExtensionApi {
  registerMcpServer(name: string, config: McpStdioServerConfig): void;
  on(event: "session_start", handler: (event: { type: "session_start" }, ctx: PiContext) => void): () => void;
  registerCommand(
    name: string,
    options: { description?: string; handler: (args: string, ctx: PiContext) => Promise<void> },
  ): void;
  exec(command: string, args: string[], options?: ExecOptions): Promise<ExecResult>;
}
