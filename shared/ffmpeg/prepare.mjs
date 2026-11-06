// Fetches FFmpeg for the package: a build without GPL parts, pinned to one file and checked against
// its hash, so that the release holds what was reviewed. Nothing to do outside Windows, where modules
// are only compiled for the checks.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const TAG = 'autobuild-2026-10-31-12-59';
const NAME = 'ffmpeg-n8.1.3-27-g8dca1586f4-win64-lgpl-8.1';
const SHA256 = 'd4c2a42673917b4a97104a3e9f082729a9730796b98664e37c0b7eba6c6c613d';

const bundle = path.join(path.dirname(fileURLToPath(import.meta.url)), 'bundle');

if (process.platform === 'win32' && !fs.existsSync(path.join(bundle, 'ffmpeg.exe'))) {
  const response = await fetch(`https://github.com/BtbN/FFmpeg-Builds/releases/download/${TAG}/${NAME}.zip`);
  if (!response.ok) throw new Error(`could not download FFmpeg: ${response.status}`);
  const archive = Buffer.from(await response.arrayBuffer());
  const hash = createHash('sha256').update(archive).digest('hex');
  if (hash !== SHA256) throw new Error(`unexpected FFmpeg archive: ${hash}`);

  const work = fs.mkdtempSync(path.join(os.tmpdir(), 'shared-ffmpeg-'));
  const zip = path.join(work, 'ffmpeg.zip');
  fs.writeFileSync(zip, archive);
  // The tar of Windows: the one Git brings reads C: as a remote host, and does not read zip files.
  const tar = path.join(process.env.SystemRoot ?? 'C:/Windows', 'System32', 'tar.exe');
  execFileSync(tar, ['-xf', zip, '-C', work], { stdio: 'inherit' });
  fs.mkdirSync(bundle, { recursive: true });
  fs.copyFileSync(path.join(work, NAME, 'bin', 'ffmpeg.exe'), path.join(bundle, 'ffmpeg.exe'));
  fs.copyFileSync(path.join(work, NAME, 'LICENSE.txt'), path.join(bundle, 'ffmpeg-LICENSE.txt'));
  fs.rmSync(work, { recursive: true, force: true });
}
