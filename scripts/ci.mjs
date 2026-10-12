import { spawn } from "node:child_process";
import { createWriteStream, mkdirSync, mkdtempSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const expectedNode = readFileSync(join(root, ".node-version"), "utf8").trim();
if (process.versions.node !== expectedNode) {
  throw new Error(`CI requires Node ${expectedNode}; received ${process.versions.node}`);
}
if (!process.env.npm_execpath) throw new Error("Run this gate with npm run ci");

mkdirSync(join(root, ".tmp"), { recursive: true });
const output = mkdtempSync(join(root, ".tmp", "ci-"));
const sandbox = join(output, "sandbox");
const paths = Object.fromEntries(["home", "appdata", "localappdata", "codex", "claude", "temp"]
  .map((name) => [name, join(sandbox, name)]));
for (const path of Object.values(paths)) mkdirSync(path, { recursive: true });
const testEnvironment = {
  ...process.env,
  // Retain build caches while isolating every default client/application path.
  CARGO_HOME: process.env.CARGO_HOME || join(homedir(), ".cargo"),
  RUSTUP_HOME: process.env.RUSTUP_HOME || join(homedir(), ".rustup"),
  HOME: paths.home,
  USERPROFILE: paths.home,
  APPDATA: paths.appdata,
  LOCALAPPDATA: paths.localappdata,
  XDG_CONFIG_HOME: join(paths.home, ".config"),
  XDG_CACHE_HOME: join(paths.home, ".cache"),
  XDG_DATA_HOME: join(paths.home, ".local", "share"),
  CODEX_HOME: paths.codex,
  CLAUDE_CONFIG_DIR: paths.claude,
  TMPDIR: paths.temp,
  TEMP: paths.temp,
  TMP: paths.temp,
};
// Live provider probes are opt-in manual checks and never belong in CI.
delete testEnvironment.ASB_RUN_LIVE_CODEX_TESTS;
for (const name of Object.keys(testEnvironment)) {
  if (name.startsWith("ASB_CODEX_LIVE_")) delete testEnvironment[name];
}
const nodeTests = readdirSync(join(root, "tests"))
  .filter((name) => name.endsWith(".test.ts")).sort().map((name) => join("tests", name));
if (!nodeTests.length) throw new Error("No frontend tests discovered");
const npm = (script) => [process.execPath, [process.env.npm_execpath, "run", script]];
const steps = [
  ["notes", ...npm("verify-notes"), process.env],
  ["frontend-tests", process.execPath, ["--test", ...nodeTests], testEnvironment],
  ["rust-tests", "cargo", ["test", "--locked", "--workspace"], testEnvironment],
  ["frontend-build", ...npm("build"), process.env],
];
const report = { node: process.versions.node, platform: process.platform, passed: false, steps: [] };

async function runStep(name, command, args, env) {
  console.log(`\n[CI] ${name}`);
  const log = createWriteStream(join(output, `${name}.log`));
  const started = Date.now();
  const code = await new Promise((resolveStep, reject) => {
    const child = spawn(command, args, { cwd: root, env, stdio: ["ignore", "pipe", "pipe"] });
    child.on("error", reject);
    child.stdout.on("data", (data) => { process.stdout.write(data); log.write(data); });
    child.stderr.on("data", (data) => { process.stderr.write(data); log.write(data); });
    child.on("close", (exitCode) => resolveStep(exitCode));
  }).finally(() => log.end());
  report.steps.push({ name, code, durationMs: Date.now() - started });
  if (code !== 0) throw new Error(`${name} failed with exit code ${code}`);
}

try {
  for (const step of steps) await runStep(...step);
  report.passed = true;
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
} finally {
  writeFileSync(join(output, "summary.json"), `${JSON.stringify(report, null, 2)}\n`);
  console.log(`\n[CI] ${report.passed ? "PASS" : "FAIL"}; logs: ${resolve(output)}`);
}
