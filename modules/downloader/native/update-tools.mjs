// Moves the module to the latest yt-dlp when there is one: its hashes must be signed by the key kept
// here, the list of sites follows, and the new program must run. Sites change their pages often, and a
// yt-dlp of a few weeks ago stops reading them: a module can only stay useful by following it.
// Tells GITHUB_OUTPUT whether something changed. Pass --force to rebuild the list of the current one.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const module = path.dirname(here);
const toolsFile = path.join(here, 'tools.json');
const manifestFile = path.join(module, 'manifest.ts');
const work = path.join(here, 'target', 'update');
const RELEASES = 'https://github.com/yt-dlp/yt-dlp/releases';
const ARCHIVE = 'yt-dlp_win.zip';

function tell(changed) {
  if (process.env.GITHUB_OUTPUT) fs.appendFileSync(process.env.GITHUB_OUTPUT, `changed=${changed}\n`);
}

async function download(url, file) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`could not download ${url}: ${response.status}`);
  fs.writeFileSync(path.join(work, file), Buffer.from(await response.arrayBuffer()));
}

async function latestTag() {
  const response = await fetch(`${RELEASES}/latest`, { redirect: 'manual' });
  const tag = response.headers.get('location')?.split('/').pop();
  if (!tag || !/^\d{4}\.\d{2}\.\d{2}(\.\d+)?$/.test(tag))
    throw new Error(`unexpected latest release: ${tag}`);
  return tag;
}

/** gpg is given names in its folder: the one of Git for Windows does not read every kind of path. */
function signedHash(tag) {
  const gpg = (...args) => execFileSync('gpg', ['--batch', ...args], { cwd: work, stdio: 'pipe' });
  fs.copyFileSync(path.join(here, 'yt-dlp-public.key'), path.join(work, 'public.key'));
  gpg('--import', 'public.key');
  gpg('--verify', 'SHA2-256SUMS.sig', 'SHA2-256SUMS');
  const line = fs
    .readFileSync(path.join(work, 'SHA2-256SUMS'), 'utf8')
    .split('\n')
    .find((row) => row.trim().endsWith(ARCHIVE));
  const hash = line?.trim().split(/\s+/)[0];
  if (!hash || !/^[0-9a-f]{64}$/.test(hash)) throw new Error(`no hash for ${ARCHIVE} in ${tag}`);
  return hash;
}

function bumpVersion() {
  const manifest = fs.readFileSync(manifestFile, 'utf8');
  const bumped = manifest.replace(
    /version: '(\d+)\.(\d+)\.(\d+)'/,
    (_, major, minor, patch) => `version: '${major}.${minor}.${Number(patch) + 1}'`,
  );
  if (bumped === manifest) throw new Error('no version in the manifest');
  fs.writeFileSync(manifestFile, bumped);
}

function listSites(tag) {
  const python = path.join(work, 'yt-dlp-sources');
  execFileSync(
    'python',
    ['-m', 'pip', 'install', '--quiet', '--no-deps', '--target', python, `yt-dlp==${tag}`],
    { stdio: 'inherit' },
  );
  execFileSync('python', ['-I', path.join(here, 'list_sites.py'), python, path.join(module, 'sites.json')], {
    stdio: 'inherit',
  });
}

const tools = JSON.parse(fs.readFileSync(toolsFile, 'utf8'));
const tag = await latestTag();
const force = process.argv.includes('--force');
if (tag === tools.ytdlp.tag && !force) {
  console.log(`already on ${tag}`);
  tell(false);
} else {
  fs.rmSync(work, { recursive: true, force: true });
  fs.mkdirSync(work, { recursive: true });
  await download(`${RELEASES}/download/${tag}/SHA2-256SUMS`, 'SHA2-256SUMS');
  await download(`${RELEASES}/download/${tag}/SHA2-256SUMS.sig`, 'SHA2-256SUMS.sig');
  tools.ytdlp = { tag, sha256: signedHash(tag) };
  fs.writeFileSync(toolsFile, `${JSON.stringify(tools, null, 2)}\n`);
  listSites(tag);

  execFileSync(process.execPath, [path.join(here, 'prepare.mjs')], { stdio: 'inherit' });
  if (process.platform === 'win32') {
    const version = execFileSync(path.join(here, 'bundle', 'yt-dlp', 'yt-dlp.exe'), ['--version'], {
      encoding: 'utf8',
    }).trim();
    console.log(`runs as ${version}`);
  }
  if (!force) bumpVersion();
  fs.rmSync(work, { recursive: true, force: true });
  tell(true);
}
