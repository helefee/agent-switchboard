// The single owner of the gate's pinned toolchain (`.node-version`). Every
// gate run executes on a Node whose version and npm health are probed; a
// missing or rotten install is re-provisioned from the dist mirror with
// checksum verification. Nothing is cached outside the repository — the
// previous manual copy in Windows Temp lost most of its npm tree and failed
// the gate silently (2026-10-14 CI rehearsal), so health is re-verified on
// every run instead of being assumed.
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { arch, platform } from "node:os";

const DIST_ROOT = (process.env.NODE_DIST_MIRROR ?? "https://nodejs.org/dist").replace(/\/+$/, "");

function distNames(version) {
  const platformLabel = platform() === "win32" ? "win"
    : platform() === "darwin" ? "darwin"
    : platform() === "linux" ? "linux"
    : null;
  if (!platformLabel) throw new Error(`Unsupported gate platform: ${platform()}`);
  const dirName = `node-v${version}-${platformLabel}-${arch()}`;
  return { dirName, file: platformLabel === "win" ? `${dirName}.zip` : `${dirName}.tar.gz` };
}

function probeNode(nodeBin, expectedVersion) {
  const probe = spawnSync(nodeBin, ["-e", "console.log(process.versions.node)"], { encoding: "utf8" });
  return probe.status === 0 && probe.stdout?.trim() === expectedVersion;
}

function probeNpm(nodeBin, npmCli) {
  return Boolean(npmCli) && spawnSync(nodeBin, [npmCli, "--version"], { encoding: "utf8" }).status === 0;
}

function healthCheck(nodeBin, npmCli, expectedVersion) {
  return probeNode(nodeBin, expectedVersion) && probeNpm(nodeBin, npmCli)
    ? { nodeBin, npmCli }
    : null;
}

/** The owned install must contain every file the checksum-verified archive
 * ships: the 2026-10-14 incident was silent file loss, and a version/npm
 * smoke probe passes a truncated tree (npm loads its CLI lazily). */
function manifestComplete(tar, archiveName, installRoot) {
  const listing = spawnSync(tar, ["-tf", archiveName], { cwd: installRoot, encoding: "utf8" });
  if (listing.status !== 0) return false;
  for (const line of listing.stdout.split("\n")) {
    const name = line.trim();
    if (!name || name.endsWith("/")) continue;
    if (!existsSync(join(installRoot, name))) return false;
  }
  return true;
}

function extractionTar() {
  if (platform() !== "win32") return "tar";
  // Git for Windows puts GNU tar on PATH first: GNU tar reads drive letters
  // as remote hosts and cannot read zip archives, so resolve the Windows
  // -shipped bsdtar explicitly.
  return join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
}

async function downloadTo(url, dest) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Toolchain download failed (${response.status}): ${url}`);
  writeFileSync(dest, Buffer.from(await response.arrayBuffer()));
}

function sha256(file) {
  return createHash("sha256").update(readFileSync(file)).digest("hex");
}

async function provision(root, version, dirName, file) {
  const installRoot = join(root, ".tmp", "node");
  const cache = join(installRoot, "archives");
  mkdirSync(cache, { recursive: true });
  const archive = join(cache, file);
  if (!existsSync(archive)) {
    console.log(`[CI] Downloading pinned toolchain ${file} from ${DIST_ROOT} ...`);
    await downloadTo(`${DIST_ROOT}/v${version}/${file}`, archive);
  }
  const shasums = join(cache, "SHASUMS256.txt");
  await downloadTo(`${DIST_ROOT}/v${version}/SHASUMS256.txt`, shasums);
  const expected = readFileSync(shasums, "utf8").split("\n")
    .map((line) => line.trim().split(/\s+/u))
    .filter((parts) => parts.length === 2 && parts[1] === file)
    .map((parts) => parts[0])[0];
  if (!expected) throw new Error(`Dist mirror lists no checksum for ${file}`);
  const actual = sha256(archive);
  if (actual !== expected) {
    rmSync(archive);
    throw new Error(`Pinned toolchain archive failed checksum: ${file} (${actual} ≠ ${expected})`);
  }
  rmSync(join(installRoot, dirName), { recursive: true, force: true });
  // Relative paths with a working directory: the extraction must never see a
  // drive letter (GNU tar reads `F:\...` as a remote host).
  const extract = spawnSync(extractionTar(), ["-xf", `archives/${file}`], { cwd: installRoot, encoding: "utf8" });
  if (extract.status !== 0) {
    throw new Error(`Toolchain extraction failed (exit ${extract.status}): ${extract.stderr ?? ""}`);
  }
}

/** Returns a healthy { nodeBin, npmCli } pair for the pinned version: the
 * repository-owned install under .tmp/node when present, else a PATH Node of
 * the exact pinned version whose own npm passes the same probe, else a fresh
 * checksum-verified download. Throws loudly when none can be established. */
export async function ensurePinnedNode(root, expectedVersion) {
  const { dirName, file } = distNames(expectedVersion);
  const installRoot = join(root, ".tmp", "node");
  const ownedDir = join(installRoot, dirName);
  const ownedBin = join(ownedDir, platform() === "win32" ? "node.exe" : join("bin", "node"));
  const ownedNpm = join(ownedDir, "node_modules", "npm", "bin", "npm-cli.js");
  const tar = extractionTar();
  if (existsSync(ownedBin)) {
    const owned = healthCheck(ownedBin, ownedNpm, expectedVersion)
      && manifestComplete(tar, `archives/${file}`, installRoot) ? { nodeBin: ownedBin, npmCli: ownedNpm } : null;
    if (owned) return owned;
    console.log(`[CI] Pinned toolchain at ${ownedDir} is broken; re-provisioning.`);
  }
  const viaPath = healthCheck("node", process.env.npm_execpath, expectedVersion);
  if (viaPath) return viaPath;
  await provision(root, expectedVersion, dirName, file);
  const fresh = healthCheck(ownedBin, ownedNpm, expectedVersion);
  if (!fresh) throw new Error(`Re-provisioned toolchain at ${ownedDir} did not pass the health check`);
  console.log(`[CI] Pinned toolchain Node v${expectedVersion} provisioned at ${ownedDir}`);
  return fresh;
}
