import qrcode from 'qrcode-generator';

export const LEVELS = ['L', 'M', 'Q', 'H'] as const;
export type Level = (typeof LEVELS)[number];

/** The most bytes a code holds at each level, in its largest version. */
export const CAPACITY: Record<Level, number> = { L: 2953, M: 2331, Q: 1663, H: 1273 };

/** The white border readers need around a code, in modules. */
export const QUIET_ZONE = 4;
/** The paper a code is printed on, whatever the theme: readers need a dark code on a light ground. */
export const PAPER = '#ffffff';
/** With the paper, the checkerboard that stands for a transparent background, as image editors show it. */
export const CHECKER = '#e5e7eb';

// Without it, the library keeps only the low byte of each character: accents would be lost.
qrcode.stringToBytes = qrcode.stringToBytesFuncs['UTF-8'] ?? qrcode.stringToBytes;

export interface Code {
  /** Modules on a side, without the quiet zone. */
  size: number;
  version: number;
  bytes: number;
  isDark: (x: number, y: number) => boolean;
  /** One square per dark module, in modules, for an SVG path. */
  path: string;
}

export const byteLength = (content: string) => new TextEncoder().encode(content).length;

/** The code of a content, `null` when there is nothing to encode or too much. */
export function encode(content: string, level: Level): Code | null {
  if (!content || byteLength(content) > CAPACITY[level]) return null;
  const qr = qrcode(0, level);
  qr.addData(content, 'Byte');
  try {
    qr.make();
  } catch {
    return null;
  }
  const size = qr.getModuleCount();
  const isDark = (x: number, y: number) => qr.isDark(y, x);
  let path = '';
  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) if (isDark(x, y)) path += `M${x} ${y}h1v1h-1z`;
  }
  return { size, version: (size - 17) / 4, bytes: byteLength(content), isDark, path };
}

/** The view box of a code with its quiet zone, in modules. */
export const viewBoxOf = (code: Code) =>
  `${-QUIET_ZONE} ${-QUIET_ZONE} ${code.size + QUIET_ZONE * 2} ${code.size + QUIET_ZONE * 2}`;

/** A standalone SVG file; `background` null leaves it transparent. */
export function svgOf(code: Code, color: string, background: string | null, pixels: number): string {
  const side = code.size + QUIET_ZONE * 2;
  const paper = background
    ? `<rect x="${-QUIET_ZONE}" y="${-QUIET_ZONE}" width="${side}" height="${side}" fill="${background}"/>`
    : '';
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" width="${pixels}" height="${pixels}" viewBox="${viewBoxOf(code)}" shape-rendering="crispEdges">` +
    `${paper}<path d="${code.path}" fill="${color}"/></svg>\n`
  );
}

/** The code drawn on a canvas of `pixels` a side, each module on whole pixels so that none blurs. */
export function canvasOf(code: Code, color: string, background: string | null, pixels: number) {
  const canvas = document.createElement('canvas');
  canvas.width = pixels;
  canvas.height = pixels;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('no 2d context');
  if (background) {
    context.fillStyle = background;
    context.fillRect(0, 0, pixels, pixels);
  }
  const scale = pixels / (code.size + QUIET_ZONE * 2);
  const edge = (module: number) => Math.round((module + QUIET_ZONE) * scale);
  context.fillStyle = color;
  for (let y = 0; y < code.size; y += 1) {
    for (let x = 0; x < code.size; x += 1) {
      if (code.isDark(x, y)) context.fillRect(edge(x), edge(y), edge(x + 1) - edge(x), edge(y + 1) - edge(y));
    }
  }
  return canvas;
}

export function pngOf(code: Code, color: string, background: string | null, pixels: number): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvasOf(code, color, background, pixels).toBlob((blob) =>
      blob ? resolve(blob) : reject(new Error('no image')),
    );
  });
}

export async function copyImage(png: Promise<Blob>): Promise<void> {
  await navigator.clipboard.write([new ClipboardItem({ 'image/png': png })]);
}
