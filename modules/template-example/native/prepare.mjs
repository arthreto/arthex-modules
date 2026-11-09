// Puts the pictures next to the program in the package. They are kept in `pictures/` rather than
// in `bundle/`, which is built and never committed.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const target = path.join(here, 'bundle', 'pictures');
fs.rmSync(target, { recursive: true, force: true });
fs.mkdirSync(target, { recursive: true });
for (const file of fs.readdirSync(path.join(here, 'pictures'))) {
  fs.copyFileSync(path.join(here, 'pictures', file), path.join(target, file));
}
