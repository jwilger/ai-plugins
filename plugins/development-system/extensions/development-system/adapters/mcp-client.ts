import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";

export type McpTool = Readonly<{
  name: string;
  description?: string;
  inputSchema: Record<string, unknown>;
}>;

export class McpClient {
  readonly #command: string;
  readonly #args: readonly string[];
  readonly #cwd: string;
  readonly #startupTimeoutMs: number;
  readonly #requestTimeoutMs: number;
  readonly #maxOutputBytes: number;
  #child: ChildProcessWithoutNullStreams | null = null;
  #buffer = "";
  #nextId = 1;
  #pending = new Map<
    number,
    {
      resolve(value: unknown): void;
      reject(error: Error): void;
      timer: NodeJS.Timeout;
    }
  >();

  constructor(
    options: Readonly<{
      command: string;
      args?: readonly string[];
      cwd: string;
      startupTimeoutMs?: number;
      requestTimeoutMs?: number;
      maxOutputBytes?: number;
    }>,
  ) {
    this.#command = options.command;
    this.#args = options.args ?? [];
    this.#cwd = options.cwd;
    this.#startupTimeoutMs = options.startupTimeoutMs ?? 5_000;
    this.#requestTimeoutMs = options.requestTimeoutMs ?? 15_000;
    this.#maxOutputBytes = options.maxOutputBytes ?? 50 * 1024;
  }

  async start(): Promise<void> {
    if (this.#child) return;
    this.#child = spawn(this.#command, [...this.#args], {
      cwd: this.#cwd,
      detached: process.platform !== "win32",
      stdio: ["pipe", "pipe", "pipe"],
    });
    this.#child.stdout.on("data", (chunk) =>
      this.#accept(chunk.toString("utf8")),
    );
    this.#child.stderr.on("data", (chunk) => {
      if (chunk.length > this.#maxOutputBytes)
        this.#failAll(new Error("development_system.component_output_limit"));
    });
    this.#child.on("error", (error) => this.#failAll(error));
    this.#child.on("exit", (code, signal) =>
      this.#failAll(
        new Error(
          `development_system.component_failed code=${code} signal=${signal}`,
        ),
      ),
    );
    await this.#request(
      "initialize",
      {
        protocolVersion: "2025-06-18",
        capabilities: {},
        clientInfo: { name: "development-system-pi", version: "1.2.0" },
      },
      this.#startupTimeoutMs,
    );
    this.#send({ jsonrpc: "2.0", method: "notifications/initialized" });
  }

  async listTools(): Promise<readonly McpTool[]> {
    const result = (await this.#request("tools/list", {})) as {
      tools?: McpTool[];
    };
    if (!Array.isArray(result.tools))
      throw new Error("development_system.mcp_tools_invalid");
    return result.tools;
  }

  async callTool(
    name: string,
    arguments_: unknown,
    signal?: AbortSignal,
  ): Promise<unknown> {
    if (signal?.aborted) throw new Error("development_system.cancelled");
    const abort = () => this.stop();
    signal?.addEventListener("abort", abort, { once: true });
    try {
      return await this.#request("tools/call", { name, arguments: arguments_ });
    } finally {
      signal?.removeEventListener("abort", abort);
    }
  }

  stop(): void {
    const child = this.#child;
    this.#child = null;
    if (!child || child.killed) return;
    if (process.platform !== "win32" && child.pid) {
      try {
        process.kill(-child.pid, "SIGTERM");
      } catch {
        child.kill("SIGTERM");
      }
    } else child.kill("SIGTERM");
    this.#failAll(new Error("development_system.component_stopped"));
  }

  #send(value: unknown): void {
    if (!this.#child?.stdin.writable)
      throw new Error("development_system.component_not_running");
    this.#child.stdin.write(`${JSON.stringify(value)}\n`);
  }

  #request(
    method: string,
    params: unknown,
    timeout = this.#requestTimeoutMs,
  ): Promise<unknown> {
    const id = this.#nextId++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.#pending.delete(id);
        reject(
          new Error(`development_system.component_timeout method=${method}`),
        );
        this.stop();
      }, timeout);
      this.#pending.set(id, { resolve, reject, timer });
      this.#send({ jsonrpc: "2.0", id, method, params });
    });
  }

  #accept(chunk: string): void {
    this.#buffer += chunk;
    if (Buffer.byteLength(this.#buffer) > this.#maxOutputBytes) {
      this.#failAll(new Error("development_system.component_output_limit"));
      this.stop();
      return;
    }
    for (;;) {
      const newline = this.#buffer.indexOf("\n");
      if (newline < 0) return;
      const line = this.#buffer.slice(0, newline).replace(/\r$/, "");
      this.#buffer = this.#buffer.slice(newline + 1);
      if (!line) continue;
      let message: {
        id?: number;
        result?: unknown;
        error?: { message?: string };
      };
      try {
        message = JSON.parse(line);
      } catch {
        this.#failAll(
          new Error("development_system.component_protocol_invalid"),
        );
        this.stop();
        return;
      }
      if (typeof message.id !== "number") continue;
      const pending = this.#pending.get(message.id);
      if (!pending) continue;
      clearTimeout(pending.timer);
      this.#pending.delete(message.id);
      if (message.error)
        pending.reject(
          new Error(
            `development_system.domain_rejection ${message.error.message ?? "MCP error"}`,
          ),
        );
      else pending.resolve(message.result);
    }
  }

  #failAll(error: Error): void {
    for (const pending of this.#pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.#pending.clear();
  }
}

export function schemaIsAdmissible(
  schema: unknown,
): schema is Record<string, unknown> {
  if (!schema || typeof schema !== "object" || Array.isArray(schema))
    return false;
  const visit = (value: unknown): boolean => {
    if (!value || typeof value !== "object") return true;
    if (Array.isArray(value)) return value.every(visit);
    const record = value as Record<string, unknown>;
    if (
      "$ref" in record ||
      "anyOf" in record ||
      "oneOf" in record ||
      "allOf" in record
    )
      return false;
    return Object.values(record).every(visit);
  };
  return visit(schema);
}

export function publicToolName(
  origin: "tiber" | "review",
  discoveredName: string,
): string {
  const suffix = discoveredName
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
  if (!suffix) throw new Error("development_system.mcp_tool_name_invalid");
  return `development_system_${origin}_${suffix}`;
}
