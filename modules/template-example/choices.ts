/** The families of formats, the options of each and what the program is asked to make. */

export type Family = 'image' | 'text' | 'code' | 'document' | 'audio' | 'video' | 'archive' | 'sized';

export const FAMILIES: readonly { id: Family; formats: readonly string[] }[] = [
  { id: 'image', formats: ['png', 'jpg', 'webp', 'gif', 'bmp', 'ico', 'tiff'] },
  { id: 'text', formats: ['txt', 'md', 'csv', 'json', 'xml', 'yaml', 'log'] },
  { id: 'code', formats: ['html', 'css', 'js', 'ts', 'py', 'sql'] },
  { id: 'document', formats: ['pdf', 'docx', 'xlsx', 'pptx', 'odt'] },
  { id: 'audio', formats: ['wav', 'mp3', 'ogg', 'flac'] },
  { id: 'video', formats: ['mp4', 'webm', 'mkv'] },
  { id: 'archive', formats: ['zip'] },
  { id: 'sized', formats: ['bin', 'dat', 'iso'] },
];

export const ALL_FORMATS = FAMILIES.flatMap((family) => family.formats);

/** A format no family knows is a file of the size chosen. */
export const familyOf = (format: string): Family =>
  FAMILIES.find((family) => family.formats.includes(format))?.id ?? 'sized';

/** What the user typed in the format field, as a format: no dot, lowercase letters and digits. */
export const cleanFormat = (text: string): string =>
  text
    .trim()
    .replace(/^\.+/, '')
    .toLowerCase()
    .replace(/[^a-z0-9]/g, '')
    .slice(0, 12);

export type NameProblem = 'empty' | 'characters' | 'reserved';

/** Why Windows would refuse `name`, as the program checks it too. */
export function nameProblem(name: string): NameProblem | null {
  const trimmed = name.trim();
  if (trimmed === '') return 'empty';
  if ([...trimmed].length > 200 || /[<>:"/\\|?*\u0000-\u001f]/.test(trimmed)) return 'characters';
  if (/^(con|prn|aux|nul|com\d|lpt\d)$/i.test(trimmed) || trimmed.endsWith('.')) return 'reserved';
  return null;
}

export const PICTURES = [
  { id: 'landscape', width: 1920, height: 1280, icon: 'M3 18l6-7 4 5 3-3 5 5M3 5h18v14H3zM16 9h.01' },
  {
    id: 'portrait',
    width: 1280,
    height: 1920,
    icon: 'M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8ZM4 21a8 8 0 0 1 16 0',
  },
  {
    id: 'city',
    width: 1920,
    height: 1280,
    icon: 'M3 21h18M5 21V9l5-3v15M10 21V4l9 4v13M14 10h2M14 14h2M7 12h1M7 16h1',
  },
  {
    id: 'animal',
    width: 1920,
    height: 1280,
    icon: 'M5 10c-1.5 0-2-1.5-2-3s1-2 2-2 2 1 2 2.5M19 10c1.5 0 2-1.5 2-3s-1-2-2-2-2 1-2 2.5M8 5c0-1.5.8-2.5 2-2.5S12 3.5 12 5M16 5c0-1.5-.8-2.5-2-2.5M7 15c0-3 2.2-5 5-5s5 2 5 5c0 2.5-2 4-5 4s-5-1.5-5-4Z',
  },
  {
    id: 'food',
    width: 1920,
    height: 1280,
    icon: 'M4 13h16a8 8 0 0 1-16 0ZM8 9c0-2 2-2 2-4M12 9c0-2 2-2 2-4M16 9c0-2 2-2 2-4',
  },
  {
    id: 'object',
    width: 1600,
    height: 1600,
    icon: 'M12 3l8 4.5v9L12 21l-8-4.5v-9ZM12 12l8-4.5M12 12v9M12 12L4 7.5',
  },
  { id: 'alpha', width: 1024, height: 1024, icon: 'M4 4h8v8H4zM12 12h8v8h-8zM12 4h8v8M4 12v8h8' },
  { id: 'text', width: 1600, height: 1000, icon: 'M5 6V4h14v2M12 4v16M9 20h6' },
] as const;
export type PictureId = (typeof PICTURES)[number]['id'];

/** `null` keeps the picture's own size. */
export const SIZES: readonly ([number, number] | null)[] = [
  null,
  [512, 512],
  [1280, 720],
  [1920, 1080],
  [3840, 2160],
];
export const ROWS = [10, 100, 1000, 100000] as const;
export const PAGES = [1, 5, 20, 100] as const;
export const SECONDS = [5, 30, 60, 600] as const;
export const HEIGHTS = [480, 720, 1080, 2160] as const;
export const FILES = [5, 50, 500] as const;
/** The formats whose number of lines the user chooses. */
export const WITH_ROWS = ['csv', 'json', 'xml', 'log'];

export const UNITS = [
  { id: 'kb', bytes: 1024 },
  { id: 'mb', bytes: 1024 ** 2 },
  { id: 'gb', bytes: 1024 ** 3 },
] as const;
export type Unit = (typeof UNITS)[number]['id'];
const MIN_BYTES = 1024;
const MAX_BYTES = 10 * 1024 ** 3;

/** The size typed, in bytes, or `null` outside 1 KB to 10 GB. A comma is a decimal point. */
export function sizeInBytes(value: string, unit: Unit): number | null {
  const number = Number(value.trim().replace(',', '.'));
  const bytes = Math.round(number * (UNITS.find((each) => each.id === unit)?.bytes ?? 1));
  return Number.isFinite(bytes) && bytes >= MIN_BYTES && bytes <= MAX_BYTES ? bytes : null;
}

/** What `.ico` keeps of a picture: 256 pixels at most on each side. */
export function iconSize(width: number, height: number): [number, number] {
  const scale = Math.min(1, 256 / Math.max(width, height));
  return [Math.round(width * scale), Math.round(height * scale)];
}

export interface Choices {
  picture: PictureId;
  size: [number, number] | null;
  rows: number;
  pages: number;
  seconds: number;
  height: number;
  files: number;
  sizeValue: string;
  unit: Unit;
  random: boolean;
}

export const DEFAULT_CHOICES: Choices = {
  picture: 'landscape',
  size: null,
  rows: 100,
  pages: 5,
  seconds: 30,
  height: 720,
  files: 5,
  sizeValue: '100',
  unit: 'mb',
  random: false,
};

/** The parameters of the program's `create`: only the options of the format's family. */
export function createRequest(folder: string, name: string, format: string, choices: Choices) {
  const base = { folder, name, format };
  switch (familyOf(format)) {
    case 'image':
      return { ...base, picture: choices.picture, size: choices.size };
    case 'text':
    case 'code':
      return WITH_ROWS.includes(format) ? { ...base, rows: choices.rows } : base;
    case 'document':
      return { ...base, pages: choices.pages };
    case 'audio':
      return { ...base, seconds: choices.seconds };
    case 'video':
      return { ...base, seconds: choices.seconds, height: choices.height };
    case 'archive':
      return { ...base, files: choices.files };
    case 'sized':
      return { ...base, bytes: sizeInBytes(choices.sizeValue, choices.unit), random: choices.random };
  }
}
