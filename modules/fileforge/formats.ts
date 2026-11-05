export type Kind = 'image' | 'video' | 'audio' | 'document';
export type Status = 'ready' | 'waiting' | 'working' | 'done' | 'failed';
export type Mode = 'convert' | 'compress';

export interface Item {
  id: number;
  path: string;
  name: string;
  kind: Kind;
  size: number;
  target: string;
  mode: Mode;
  /** The batch it was started with. */
  run: number;
  status: Status;
  progress: number;
  output: string | null;
  outputSize: number | null;
  noGain: boolean;
  error: 'needsOffice' | 'failed' | null;
}

export interface Settings {
  quality: number;
  height: number | null;
  stripMetadata: boolean;
  level: number;
  folder: string | null;
}

export interface QueueState {
  items: Item[];
  mode: Mode;
  settings: Settings;
}

export const IMAGE_FORMATS = ['jpeg', 'png', 'webp', 'avif', 'ico', 'gif', 'tiff', 'bmp'];
export const VIDEO_FORMATS = ['mp4', 'webm', 'mov', 'mkv', 'avi', 'gif'];
export const AUDIO_FORMATS = ['mp3', 'm4a', 'wav', 'flac', 'ogg'];
export const DOCUMENT_FORMATS = ['pdf', 'docx', 'odt', 'txt', 'html'];

const DISPLAY: Record<string, string> = { webp: 'WebP', webm: 'WebM' };

export const formatLabel = (format: string): string => DISPLAY[format] ?? format.toUpperCase();

export type FormatGroup = { group: Kind; formats: string[] };

/** What a file of this kind can become. A video can also become its sound. */
export function groupsOf(kind: Kind): FormatGroup[] {
  switch (kind) {
    case 'image':
      return [{ group: 'image', formats: IMAGE_FORMATS }];
    case 'video':
      return [
        { group: 'video', formats: VIDEO_FORMATS },
        { group: 'audio', formats: AUDIO_FORMATS },
      ];
    case 'audio':
      return [{ group: 'audio', formats: AUDIO_FORMATS }];
    case 'document':
      return [{ group: 'document', formats: DOCUMENT_FORMATS }];
  }
}

export const extensionOf = (name: string): string => name.split('.').pop()?.toUpperCase() ?? '';

const ended = (item: Item) => item.status === 'done' || item.status === 'failed';

/** Over the latest batch only: files finished earlier must not count when new ones start. */
export function totals(items: readonly Item[]) {
  const started = items.filter((item) => item.status !== 'ready');
  const latest = Math.max(0, ...started.map((item) => item.run));
  const run = started.filter((item) => item.run === latest);
  const done = run.filter(ended).length;
  const sum = run.reduce((total, item) => total + (ended(item) ? 100 : item.progress), 0);
  const saved = run.reduce(
    (total, item) => total + (item.outputSize === null ? 0 : Math.max(0, item.size - item.outputSize)),
    0,
  );
  return {
    count: run.length,
    done,
    percent: run.length === 0 ? 0 : Math.floor(sum / run.length),
    running: run.some((item) => !ended(item)),
    saved,
  };
}

/** Minutes left at the pace so far, at least one, or `null` too early to say. */
export function minutesLeft(elapsedMs: number, percent: number): number | null {
  if (percent < 3 || percent >= 100) return null;
  return Math.max(1, Math.ceil((elapsedMs * (100 - percent)) / percent / 60_000));
}
