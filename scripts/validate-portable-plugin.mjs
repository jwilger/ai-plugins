#!/usr/bin/env node
// Validate vendored Agent Plugins 1.0.0 schemas and package-relative launchers.
import fs from "node:fs";
import path from "node:path";
import Ajv2020 from "ajv/dist/2020.js";

const root = path.resolve(process.argv[2] || path.join(import.meta.dirname, ".."));
const schemas = path.join(import.meta.dirname, "schemas/agent-plugins-1.0.0");
const ajv = new Ajv2020({ allErrors: true, strict: false });
const pluginSchema = JSON.parse(fs.readFileSync(path.join(schemas, "plugin.schema.json"), "utf8"));
const mcpSchema = JSON.parse(fs.readFileSync(path.join(schemas, "mcp.schema.json"), "utf8"));
const validatePlugin = ajv.compile(pluginSchema);
const validateMcp = ajv.compile(mcpSchema);

function fail(name, detail) {
  throw new Error(`portable-schema: ${name}: ${detail}`);
}
function parse(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}
function contained(rootPath, relative) {
  const resolved = path.resolve(rootPath, relative);
  return resolved.startsWith(`${rootPath}${path.sep}`);
}
for (const name of fs.readdirSync(path.join(root, "plugins"))) {
  const pluginRoot = path.join(root, "plugins", name);
  if (!fs.statSync(pluginRoot).isDirectory()) continue;
  const manifest = parse(path.join(pluginRoot, "plugin.json"));
  if (!validatePlugin(manifest)) fail(name, ajv.errorsText(validatePlugin.errors));
  if (manifest.name !== name) fail(name, "manifest name differs from directory");
  const mcpFile = path.join(pluginRoot, "mcp.json");
  if (!fs.existsSync(mcpFile)) continue;
  const mcp = parse(mcpFile);
  if (!validateMcp(mcp)) fail(name, ajv.errorsText(validateMcp.errors));
  for (const [serverName, server] of Object.entries(mcp.mcpServers)) {
    if (server.type === "stdio") {
      if (server.command.startsWith("./")) {
        if (!contained(pluginRoot, server.command)) fail(name, `${serverName} command escapes plugin root`);
        const target = path.resolve(pluginRoot, server.command);
        const stat = fs.statSync(target, { throwIfNoEntry: false });
        if (!stat?.isFile() || (stat.mode & 0o111) === 0) fail(name, `${serverName} launcher is missing or not executable`);
        if (!contained(fs.realpathSync.native(pluginRoot), path.relative(fs.realpathSync.native(pluginRoot), fs.realpathSync.native(target)))) {
          fail(name, `${serverName} launcher resolves outside plugin root`);
        }
      } else if (server.command.includes("/") || /\s/.test(server.command)) {
        fail(name, `${serverName} command must be a bare executable token or plugin-relative path`);
      }
      if (server.cwd) {
        const cwd = server.cwd;
        const relative = cwd.startsWith("./") ? cwd.slice(2) : cwd.replace(/^\$\{(?:PLUGIN_ROOT|PLUGIN_DATA)\}\/?/, "");
        if (relative.split("/").includes("..") || relative.includes("\\")) fail(name, `${serverName} cwd escapes its root`);
        if (!cwd.startsWith("${PLUGIN_DATA}") && relative) {
          if (!contained(pluginRoot, relative)) fail(name, `${serverName} cwd escapes plugin root`);
          const target = path.resolve(pluginRoot, relative);
          if (fs.existsSync(target) && !contained(fs.realpathSync.native(pluginRoot), path.relative(fs.realpathSync.native(pluginRoot), fs.realpathSync.native(target)))) {
            fail(name, `${serverName} cwd resolves outside plugin root`);
          }
        }
      }
    } else {
      let url;
      try { url = new URL(server.url); } catch { fail(name, `${serverName} URL is invalid`); }
      const loopback = ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname);
      if (!(url.protocol === "https:" || (url.protocol === "http:" && loopback)) || url.username || url.password || url.hash) {
        fail(name, `${serverName} URL must use HTTPS or loopback HTTP without credentials or fragments`);
      }
    }
  }
}
console.log("portable-schema: ok");
