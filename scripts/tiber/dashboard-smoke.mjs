#!/usr/bin/env node

import { spawn, spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium } from "playwright";

const root = resolve(new URL("../..", import.meta.url).pathname);
const manifestPath = join(root, "plugins/tiber/rust/Cargo.toml");
const repo = mkdtempSync(join(tmpdir(), "tiber-dashboard-smoke-"));
const port = await findFreePort();
const url = `http://127.0.0.1:${port}/`;
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
  tiber(["create", "Blocked dashboard task"]);
  tiber(["link", "inspect-dashboard", "blocks", "blocked-dashboard-task"]);
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
    {
      cwd: repo,
      env: { ...process.env, TIBER_DASHBOARD_PORT: String(port) },
      stdio: ["ignore", "pipe", "pipe"],
    },
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

  const inspectCard = page.locator('[data-task-link][data-stem$="-inspect-dashboard"]');
  const blockedCard = page.locator('[data-task-link][data-stem$="-blocked-dashboard-task"]');
  await inspectCard.click();
  await blockedCard.waitFor();
  assert(
    (await blockedCard.getAttribute("class")).includes("is-dependent"),
    "single-click selection should highlight directly blocked tasks",
  );
  await assertText(blockedCard, "blocks");

  await inspectCard.dblclick();
  await page.locator("[data-task-modal][open]").waitFor();
  await assertText(page.locator("[data-modal-content]"), "Inspect dashboard");
  await assertText(page.locator("[data-modal-content]"), "Blocked dashboard task");
  await page.locator("[data-modal-close]").click();

  await page.getByRole("link", { name: "Docs" }).click();
  await page.waitForURL("**/docs");
  await assertText(page.locator("body"), "docs/guides/tiber.md");
  await page.getByRole("link", { name: "docs/guides/tiber.md" }).click();
  await page.waitForURL("**/docs/guides/tiber.md");
  await assertText(page.locator(".docs-content"), "Tiber guide");
  assert(
    (await page.locator(".docs-content h1").textContent()) === "Tiber guide",
    "docs markdown should render the document heading",
  );

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

async function findFreePort() {
  return new Promise((resolvePort, reject) => {
    const probe = createServer();
    probe.on("error", reject);
    probe.listen(0, "127.0.0.1", () => {
      const address = probe.address();
      probe.close(() => {
        if (address && typeof address === "object") {
          resolvePort(address.port);
        } else {
          reject(new Error("could not allocate dashboard smoke port"));
        }
      });
    });
  });
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
  const timeoutMs = 5000;
  const deadline = Date.now() + timeoutMs;
  let text = "";
  while (Date.now() < deadline) {
    text = (await locator.textContent({ timeout: 1000 })) ?? "";
    if (text.includes(expected)) {
      return;
    }
    await new Promise((resolveSleep) => setTimeout(resolveSleep, 100));
  }
  assert(false, `expected text ${JSON.stringify(expected)} in ${JSON.stringify(text)}`);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
