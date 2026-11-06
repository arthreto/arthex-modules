const MAX_LENGTH = 2048;
const HIGHEST_DEFAULT_HEIGHT = 1080;
const DEFAULT_BITRATE = 192;

export const AUDIO_FORMATS = ['mp3', 'm4a', 'opus', 'flac', 'wav'] as const;

export interface VideoQuality {
  height: number;
  size: number | null;
}

export interface AudioQuality {
  bitrate: number;
  size: number | null;
}

/** What is typed as a full address of the web, or nothing. The scheme can be left out. */
export function toLink(text: string): string | null {
  const typed = text.trim();
  if (!typed || typed.length > MAX_LENGTH || /\s/.test(typed)) return null;
  const hasScheme = /^[a-z][a-z0-9+.-]*:\/\//i.test(typed);
  if (hasScheme && !/^https?:\/\//i.test(typed)) return null;
  try {
    const url = new URL(hasScheme ? typed : `https://${typed}`);
    return url.hostname.includes('.') ? url.href : null;
  } catch {
    return null;
  }
}

/** The site of a link that leads to something, when it is one of `sites`: a home page has no video. */
export function knownSite(link: string, sites: ReadonlySet<string>): string | null {
  const url = new URL(link);
  if (url.pathname.length <= 1 && !url.search) return null;
  const host = url.hostname.replace(/^www\./, '');
  const parts = host.split('.');
  for (let start = 0; start < parts.length - 1; start += 1) {
    if (sites.has(parts.slice(start).join('.'))) return host;
  }
  return null;
}

/** 635 becomes 10:35, and 3 700 becomes 1:01:40. */
export function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const rest = String(Math.floor(seconds % 60)).padStart(2, '0');
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, '0')}:${rest}` : `${minutes}:${rest}`;
}

export const defaultHeight = (videos: readonly VideoQuality[]): number =>
  (videos.find((video) => video.height <= HIGHEST_DEFAULT_HEIGHT) ?? videos[0])?.height ?? 0;

export const defaultBitrate = (audios: readonly AudioQuality[]): number =>
  (audios.find((audio) => audio.bitrate === DEFAULT_BITRATE) ?? audios[0])?.bitrate ?? DEFAULT_BITRATE;

/** They keep every sound, so there is no quality to choose. */
export const isLossless = (format: string): boolean => format === 'flac' || format === 'wav';

export const fileName = (path: string): string => path.split(/[\\/]/).pop() ?? path;
