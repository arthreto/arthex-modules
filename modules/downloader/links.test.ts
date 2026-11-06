import { describe, expect, it } from 'vitest';
import {
  defaultBitrate,
  defaultHeight,
  fileName,
  formatDuration,
  isLossless,
  knownSite,
  toLink,
} from './links';

const SITES = new Set(['youtube.com', 'youtu.be', 'bbc.co.uk', 'tiktok.com']);

describe('toLink', () => {
  it('reads an address, with or without its scheme', () => {
    expect(toLink('https://youtu.be/abc')).toBe('https://youtu.be/abc');
    expect(toLink('  youtube.com/watch?v=abc ')).toBe('https://youtube.com/watch?v=abc');
    expect(toLink('HTTP://Exemple.fr/a')).toBe('http://exemple.fr/a');
  });

  it('refuses the rest', () => {
    for (const text of [
      '',
      'bonjour',
      'deux mots.fr',
      'ftp://a.fr/b',
      'file:///c:/a.mp4',
      'https://',
      '--exec=a',
    ]) {
      expect(toLink(text), text).toBeNull();
    }
    expect(toLink(`https://a.fr/${'a'.repeat(3000)}`)).toBeNull();
  });
});

describe('knownSite', () => {
  it('finds the site of a link, whatever its subdomain', () => {
    expect(knownSite('https://www.youtube.com/watch?v=a', SITES)).toBe('youtube.com');
    expect(knownSite('https://music.youtube.com/watch?v=a', SITES)).toBe('music.youtube.com');
    expect(knownSite('https://vm.tiktok.com/abc', SITES)).toBe('vm.tiktok.com');
    expect(knownSite('https://www.bbc.co.uk/iplayer/a', SITES)).toBe('bbc.co.uk');
  });

  it('leaves out the other sites and the home pages', () => {
    expect(knownSite('https://exemple.fr/video', SITES)).toBeNull();
    expect(knownSite('https://notyoutube.com/video', SITES)).toBeNull();
    expect(knownSite('https://co.uk/video', SITES)).toBeNull();
    expect(knownSite('https://www.youtube.com/', SITES)).toBeNull();
  });
});

describe('formatDuration', () => {
  it('counts minutes, then hours', () => {
    expect(formatDuration(5)).toBe('0:05');
    expect(formatDuration(635)).toBe('10:35');
    expect(formatDuration(3700)).toBe('1:01:40');
  });
});

describe('defaults', () => {
  it('stops at 1080p unless there is nothing lower', () => {
    const size = null;
    expect(
      defaultHeight([
        { height: 2160, size },
        { height: 1080, size },
        { height: 720, size },
      ]),
    ).toBe(1080);
    expect(
      defaultHeight([
        { height: 2160, size },
        { height: 720, size },
      ]),
    ).toBe(720);
    expect(defaultHeight([{ height: 2160, size }])).toBe(2160);
    expect(defaultHeight([])).toBe(0);
  });

  it('starts sound at 192 kbit/s', () => {
    const size = null;
    expect(
      defaultBitrate([
        { bitrate: 320, size },
        { bitrate: 192, size },
      ]),
    ).toBe(192);
    expect(defaultBitrate([{ bitrate: 128, size }])).toBe(128);
  });

  it('knows which formats keep every sound', () => {
    expect(isLossless('flac')).toBe(true);
    expect(isLossless('mp3')).toBe(false);
  });
});

describe('fileName', () => {
  it('keeps the end of a path of any system', () => {
    expect(fileName('C:\\Users\\a\\Video.mp4')).toBe('Video.mp4');
    expect(fileName('/home/a/Video.mp4')).toBe('Video.mp4');
  });
});
