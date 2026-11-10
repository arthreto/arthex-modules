// Puts the libraries of sherpa-onnx next to the program in the package. Its build copies them next
// to the program it makes, on Windows; elsewhere the program finds them where they were unpacked.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const LIBRARIES = ['sherpa-onnx-c-api.dll', 'onnxruntime.dll', 'onnxruntime_providers_shared.dll'];

const here = path.dirname(fileURLToPath(import.meta.url));
const built = path.join(here, 'target', 'release');
const target = path.join(here, 'bundle');
fs.rmSync(target, { recursive: true, force: true });
fs.mkdirSync(target, { recursive: true });
for (const library of LIBRARIES) {
  const from = path.join(built, library);
  if (fs.existsSync(from)) fs.copyFileSync(from, path.join(target, library));
}
