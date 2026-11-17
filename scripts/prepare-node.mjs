// Bundle the official Node distribution, including npm and its licenses.
// Hashes come from https://nodejs.org/dist/v24.22.3/SHASUMS256.txt.
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const version = '24.22.3';
const distributions = {
  'aarch64-apple-darwin': ['darwin-arm64.tar.gz', 'ff6e17d4874b76d272212b908cc020d6e815fba5a6c7eaaa06add725840307fd'],
  'x86_64-apple-darwin': ['darwin-x64.tar.gz', '977614af0504c81045e7fd85596dbca0e5f951b96681516dadafedcca8c447f7'],
  'x86_64-pc-windows-msvc': ['win-x64.zip', 'e612af99f626ab415ea225af6bce5c971feb448cb5cae250bb1c3fa50c5f7d9f'],
  'aarch64-pc-windows-msvc': ['win-arm64.zip', 'b3cc74a295b39d4f273271e96f58c06dd2b35757ca12abec64c7d0598b434f14'],
};
const hostArch = { arm64: 'aarch64', x64: 'x86_64' }[process.arch];
const hostTarget = process.platform === 'darwin' ? `${hostArch}-apple-darwin` : `${hostArch}-pc-windows-msvc`;
const target = process.argv[2] || process.env.TAURI_ENV_TARGET_TRIPLE || hostTarget;
const distribution = distributions[target];
if (!distribution) throw new Error(`No bundled Node distribution for ${target}.`);
const [suffix, checksum] = distribution;
const windows = suffix.startsWith('win-');
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const destination = join(root, 'src-tauri', 'toolchains', 'node');
const receipt = JSON.stringify({ format: 1, version, target, checksum });
const marker = join(destination, 'relay-node.json');
const nodePath = join(destination, windows ? 'node.exe' : 'bin/node');
if (existsSync(marker) && readFileSync(marker, 'utf8') === receipt && existsSync(nodePath)
    && existsSync(join(destination, windows ? 'node_modules/npm/bin/npm-cli.js' : 'lib/node_modules/npm/bin/npm-cli.js'))) {
  console.log(`Bundled Node ${version} ready (${target}).`);
} else {
  const work = mkdtempSync(join(tmpdir(), 'relay-node-'));
  try {
    const name = `node-v${version}-${suffix}`;
    console.log(`Preparing Node ${version} and npm (${target})…`);
    const response = await fetch(`https://nodejs.org/dist/v${version}/${name}`, { signal: AbortSignal.timeout(180_000) });
    if (!response.ok) throw new Error(`Node download failed: HTTP ${response.status}`);
    const bytes = Buffer.from(await response.arrayBuffer());
    if (createHash('sha256').update(bytes).digest('hex') !== checksum) throw new Error('Node download checksum mismatch.');
    const archive = join(work, name);
    writeFileSync(archive, bytes);
    // macOS and supported Windows versions provide tar (including ZIP support).
    execFileSync('tar', ['-xf', archive, '-C', work], { stdio: 'inherit' });
    const extracted = join(work, `node-v${version}-${suffix.replace(/\.(tar\.gz|zip)$/, '')}`);
    const staged = join(work, 'node');
    mkdirSync(staged);
    // Keep extensionless npm/npx entrypoints in CommonJS even when building
    // Relay from this repository's ES module package scope.
    writeFileSync(join(staged, 'package.json'), '{"private":true,"type":"commonjs"}\n');
    cpSync(join(extracted, 'LICENSE'), join(staged, 'LICENSE'));
    if (windows) {
      for (const name of ['node.exe', 'npm', 'npm.cmd', 'npx', 'npx.cmd', 'node_modules']) {
        cpSync(join(extracted, name), join(staged, name), { recursive: true });
      }
    } else {
      mkdirSync(join(staged, 'bin'));
      mkdirSync(join(staged, 'lib', 'node_modules'), { recursive: true });
      cpSync(join(extracted, 'bin', 'node'), join(staged, 'bin', 'node'));
      cpSync(join(extracted, 'lib', 'node_modules', 'npm'), join(staged, 'lib', 'node_modules', 'npm'), { recursive: true });
      // Real files survive Tauri's resource copy and relocation inside an app.
      for (const tool of ['npm', 'npx']) {
        writeFileSync(join(staged, 'bin', tool), `#!/usr/bin/env node\nrequire('../lib/node_modules/npm/bin/${tool}-cli.js');\n`, { mode: 0o755 });
      }
    }
    writeFileSync(join(staged, 'relay-node.json'), receipt);
    mkdirSync(dirname(destination), { recursive: true });
    // Copy rather than rename: the temporary folder may be on another volume.
    const pending = `${destination}.pending`;
    rmSync(pending, { recursive: true, force: true });
    cpSync(staged, pending, { recursive: true });
    rmSync(destination, { recursive: true, force: true });
    renameSync(pending, destination);
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}
