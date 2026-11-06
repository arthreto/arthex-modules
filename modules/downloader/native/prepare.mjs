// Fetches yt-dlp and QuickJS for the package: pinned in tools.json and checked against their hashes,
// so that the release holds what was reviewed and every program in it is signed with the module.
// Nothing to do outside Windows, where the program is only compiled for the checks.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const bundle = path.join(here, 'bundle');
const tools = JSON.parse(fs.readFileSync(path.join(here, 'tools.json'), 'utf8'));

async function fetchChecked(url, sha256) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`could not download ${url}: ${response.status}`);
  const content = Buffer.from(await response.arrayBuffer());
  const hash = createHash('sha256').update(content).digest('hex');
  if (hash !== sha256) throw new Error(`unexpected file ${url}: ${hash}`);
  return content;
}

if (process.platform === 'win32') {
  // Beside the bundle, which goes into the package as it is.
  const stamp = path.join(here, 'target', 'tools.stamp');
  const wanted = JSON.stringify(tools);
  const ready = fs.existsSync(stamp) && fs.readFileSync(stamp, 'utf8') === wanted;
  if (!ready) {
    fs.rmSync(bundle, { recursive: true, force: true });
    fs.mkdirSync(path.join(bundle, 'yt-dlp'), { recursive: true });

    const archive = await fetchChecked(
      `https://github.com/yt-dlp/yt-dlp/releases/download/${tools.ytdlp.tag}/yt-dlp_win.zip`,
      tools.ytdlp.sha256,
    );
    const work = fs.mkdtempSync(path.join(os.tmpdir(), 'downloader-yt-dlp-'));
    const zip = path.join(work, 'yt-dlp.zip');
    fs.writeFileSync(zip, archive);
    // The tar of Windows: the one Git brings reads C: as a remote host, and does not read zip files.
    const tar = path.join(process.env.SystemRoot ?? 'C:/Windows', 'System32', 'tar.exe');
    execFileSync(tar, ['-xf', zip, '-C', path.join(bundle, 'yt-dlp')], { stdio: 'inherit' });
    fs.rmSync(work, { recursive: true, force: true });

    const quickjs = await fetchChecked(
      `https://github.com/quickjs-ng/quickjs/releases/download/${tools.quickjs.tag}/qjs-windows-x86_64.exe`,
      tools.quickjs.sha256,
    );
    fs.writeFileSync(path.join(bundle, 'qjs.exe'), quickjs);
    const license = await fetchChecked(
      `https://raw.githubusercontent.com/quickjs-ng/quickjs/${tools.quickjs.tag}/LICENSE`,
      tools.quickjs.license,
    );
    fs.writeFileSync(path.join(bundle, 'qjs-LICENSE.txt'), license);
    fs.mkdirSync(path.dirname(stamp), { recursive: true });
    fs.writeFileSync(stamp, wanted);
  }
}
