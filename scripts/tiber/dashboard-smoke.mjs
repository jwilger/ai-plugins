#!/usr/bin/env node

import { spawn, spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium } from "playwright";

const root = resolve(new URL("../..", import.meta.url).pathname);
const manifestPath = join(root, "plugins/tiber/rust/Cargo.toml");
const repo = mkdtempSync(join(tmpdir(), "tiber-dashboard-smoke-"));
const url = "http://127.0.0.1:7417/";
let server;
let browser;

try {
  run("git", ["init", "-b", "main"], repo);
  run("git", ["config", "user.email", "tiber@example.test"], repo);
  run("git", ["config", "user.name", "Tiber Smoke"], repo);
  run("git", ["config", "commit.gpgsign", "false"], repo);
  writeFileSync(join(repo, "README.md"), "# smoke repo\n");
  run("git", ["add", "README.md"], repo);
  run("git", ["commit", "-m", "Initial commit"], repo);
  mkdirSync(join(repo, "docs/guides"), { recursive: true });
  writeFileSync(
    join(repo, "docs/guides/tiber.md"),
    "# Tiber guide\n\nDashboard docs stay read-only.\n",
  );

  tiber(["init"]);
  tiber(["create", "Inspect dashboard"]);
  run(
    "cargo",
    ["build", "--quiet", "--manifest-path", manifestPath, "--bin", "tiber"],
    repo,
  );

  server = spawn(
    "cargo",
    [
      "run",
      "--quiet",
      "--manifest-path",
      manifestPath,
      "--bin",
      "tiber",
      "--",
      "dashboard",
      "serve",
    ],
    { cwd: repo, stdio: ["ignore", "pipe", "pipe"] },
  );
  server.stderr.on("data", (chunk) => process.stderr.write(chunk));
  server.stdout.on("data", (chunk) => process.stdout.write(chunk));

  await waitForDashboard(url);

  browser = await chromium.launch({
    executablePath: findBrowserExecutable(),
  });
  const page = await browser.newPage({
    viewport: { width: 1280, height: 800 },
  });
  await page.goto(url);
  await assertText(page.locator("[data-dashboard-board]"), "Inspect dashboard");

  await page.getByText("todo/inspect-dashboard.md Inspect dashboard").click();
  await page.locator("[data-task-modal][open]").waitFor();
  await assertText(page.locator("[data-modal-content]"), "# Inspect dashboard");
  await page.locator("[data-modal-close]").click();

  await page.getByRole("link", { name: "Docs" }).click();
  await page.waitForURL("**/docs");
  await assertText(page.locator("body"), "docs/guides/tiber.md");

  await page.goto(url);
  await page.locator("[data-external-link]").click();
  assert(
    page.url() === url,
    `external link should be intercepted without navigation, got ${page.url()}`,
  );
  await assertText(page.locator("[data-link-intercept-status]"), "intercepted");

  mkdirSync(join(root, "output/playwright"), { recursive: true });
  await page.screenshot({
    path: join(root, "output/playwright/tiber-dashboard.png"),
    fullPage: true,
  });
} finally {
  if (browser) {
    await browser.close();
  }
  if (server) {
    server.kill("SIGTERM");
  }
  rmSync(repo, { recursive: true, force: true });
}

function tiber(args) {
  run(
    "cargo",
    [
      "run",
      "--quiet",
      "--manifest-path",
      manifestPath,
      "--bin",
      "tiber",
      "--",
      ...args,
    ],
    repo,
  );
}

function run(command, args, cwd) {
  const output = spawnSync(command, args, {
    cwd,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (output.status !== 0) {
    throw new Error(
      [
        `command failed: ${command} ${args.join(" ")}`,
        `status: ${output.status}`,
        `stdout:\n${output.stdout}`,
        `stderr:\n${output.stderr}`,
      ].join("\n"),
    );
  }
}

function findBrowserExecutable() {
  if (process.env.TIBER_DASHBOARD_CHROME) {
    return process.env.TIBER_DASHBOARD_CHROME;
  }
  for (const command of [
    "chromium",
    "chromium-browser",
    "google-chrome",
    "google-chrome-stable",
  ]) {
    const output = spawnSync("command", ["-v", command], {
      encoding: "utf8",
      shell: true,
    });
    if (output.status === 0 && output.stdout.trim()) {
      return output.stdout.trim();
    }
  }
  return undefined;
}

async function waitForDashboard(targetUrl) {
  const timeoutMs = 60000;
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (server.exitCode !== null) {
      throw new Error(`dashboard server exited with status ${server.exitCode}`);
    }
    try {
      const remainingMs = Math.max(1, deadline - Date.now());
      const signal = AbortSignal.timeout(Math.min(1000, remainingMs));
      const response = await fetch(targetUrl, { signal });
      if (response.ok) {
        return;
      }
    } catch (_error) {
      // Server is still starting.
    }
    await new Promise((resolveSleep) => setTimeout(resolveSleep, 100));
  }
  throw new Error(`dashboard server did not start within ${timeoutMs / 1000}s`);
}

async function assertText(locator, expected) {
  const text = await locator.textContent({ timeout: 5000 });
  assert(
    text?.includes(expected),
    `expected text ${JSON.stringify(expected)} in ${JSON.stringify(text)}`,
  );
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
