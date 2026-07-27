import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "../..");
const pinnedPi =
  process.env.PI_EVAL_BIN ?? path.join(root, "node_modules/.bin/pi");

export default class PiProvider {
  constructor(options = {}) {
    this.options = options;
  }
  id() {
    return this.options.id ?? "pi:json";
  }

  async callApi(prompt) {
    const config = this.options.config ?? {};
    const agentDirectory = config.agent_dir;
    if (!path.isAbsolute(agentDirectory ?? ""))
      return { error: "Pi provider requires an absolute isolated agent_dir" };
    const compositionFile = path.join(agentDirectory, "composition.json");
    if (!fs.existsSync(compositionFile))
      return { error: "Pi provider agent home is not prepared" };
    const composition = JSON.parse(fs.readFileSync(compositionFile, "utf8"));
    if (composition.mode !== config.package_mode)
      return { error: "Pi provider package composition mismatch" };
    const marker = path.join(
      agentDirectory,
      `extension-marker-${process.pid}-${Date.now()}.json`,
    );
    const args = [
      "--mode",
      "json",
      "--no-session",
      "--approve",
      "--provider",
      config.provider ?? "openai-codex",
      "--model",
      config.model ?? "gpt-5.6-terra",
      "--thinking",
      config.thinking ?? "medium",
      prompt,
    ];
    const timeoutMs = Number(config.timeout_ms ?? 600_000);
    return new Promise((resolve) => {
      const child = spawn(pinnedPi, args, {
        cwd: config.working_dir ?? root,
        detached: process.platform !== "win32",
        env: {
          ...process.env,
          PI_CODING_AGENT_DIR: agentDirectory,
          PI_CODING_AGENT_SESSION_DIR: path.join(agentDirectory, "sessions"),
          PI_OFFLINE: "1",
          PI_TELEMETRY: "0",
          DEVELOPMENT_SYSTEM_PI_EVAL_MARKER:
            composition.mode === "no-plugins" ? "" : marker,
        },
        stdio: ["ignore", "pipe", "pipe"],
      });
      let stdout = "";
      let stderr = "";
      let settled = false;
      const stop = () => {
        if (process.platform !== "win32" && child.pid) {
          try {
            process.kill(-child.pid, "SIGKILL");
          } catch {
            child.kill("SIGKILL");
          }
        } else child.kill("SIGKILL");
      };
      const finish = (response) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        resolve(response);
      };
      const timer = setTimeout(() => {
        stop();
        finish({ error: "Pi provider timed out" });
      }, timeoutMs);
      child.stdout.on("data", (chunk) => {
        stdout += chunk.toString("utf8");
        if (Buffer.byteLength(stdout) > 5 * 1024 * 1024) {
          stop();
          finish({ error: "Pi provider output exceeded 5MB" });
        }
      });
      child.stderr.on("data", (chunk) => {
        stderr += chunk.toString("utf8");
      });
      child.on("error", (error) =>
        finish({ error: `Pi provider failed: ${error.message}` }),
      );
      child.on("close", (code) => {
        if (settled) return;
        if (code !== 0) {
          finish({
            error: `Pi provider exited ${code}: ${stderr.slice(0, 1000)}`,
          });
          return;
        }
        const records = [];
        try {
          for (const line of stdout.split("\n"))
            if (line.trim()) records.push(JSON.parse(line));
        } catch (error) {
          finish({
            error: `Pi provider emitted invalid JSONL: ${error.message}`,
          });
          return;
        }
        const assistant = [...records]
          .reverse()
          .find(
            (record) =>
              record.type === "message_end" &&
              record.message?.role === "assistant",
          )?.message;
        if (!assistant) {
          finish({ error: "Pi provider emitted no final assistant message" });
          return;
        }
        const output = (
          Array.isArray(assistant.content) ? assistant.content : []
        )
          .filter((item) => item.type === "text")
          .map((item) => item.text)
          .join("\n");
        const provenance = fs.existsSync(marker)
          ? JSON.parse(fs.readFileSync(marker, "utf8"))
          : null;
        if (composition.mode === "no-plugins" && provenance) {
          finish({ error: "Pi no-package mode executed package extension" });
          return;
        }
        if (composition.mode !== "no-plugins" && !provenance) {
          finish({ error: "Pi package extension provenance missing" });
          return;
        }
        finish({
          output,
          tokenUsage: assistant.usage
            ? {
                prompt: assistant.usage.input ?? 0,
                completion: assistant.usage.output ?? 0,
                total:
                  assistant.usage.totalTokens ??
                  (assistant.usage.input ?? 0) + (assistant.usage.output ?? 0),
              }
            : undefined,
          cost: assistant.usage?.cost?.total,
          metadata: {
            harness: "pi",
            provider: config.provider ?? "openai-codex",
            model: config.model ?? "gpt-5.6-terra",
            packageMode: composition.mode,
            packages: composition.packages,
            extensionProvenance: provenance,
            toolTrajectory: records
              .filter((record) => record.type === "tool_execution_end")
              .map((record) => ({
                name: record.toolName,
                isError: record.isError,
              })),
          },
          raw: JSON.stringify({ records, composition, provenance }),
        });
      });
    });
  }
}
