import { describe, expect, it } from 'vitest';
import { extensionOf, formatLabel, groupsOf, minutesLeft, totals, type Item } from './formats';

function item(changes: Partial<Item>): Item {
  return {
    id: 1,
    path: 'a.png',
    name: 'a.png',
    kind: 'image',
    size: 100,
    target: 'jpeg',
    mode: 'convert',
    run: 1,
    status: 'ready',
    progress: 0,
    output: null,
    outputSize: null,
    noGain: false,
    error: null,
    ...changes,
  };
}

describe('formats', () => {
  it('lets a video become its sound, and not the reverse', () => {
    const video = groupsOf('video').flatMap((group) => group.formats);
    expect(video).toContain('mp3');
    expect(groupsOf('audio').flatMap((group) => group.formats)).not.toContain('mp4');
  });

  it('offers icons for images', () => {
    expect(groupsOf('image')[0]?.formats).toContain('ico');
  });

  it('writes names as people know them', () => {
    expect(formatLabel('jpeg')).toBe('JPEG');
    expect(formatLabel('webp')).toBe('WebP');
    expect(extensionOf('a.b.mp4')).toBe('MP4');
  });
});

describe('totals', () => {
  it('counts only the files of the run', () => {
    const result = totals([
      item({ status: 'done', progress: 100, outputSize: 40 }),
      item({ status: 'working', progress: 50 }),
      item({ status: 'ready' }),
    ]);
    expect(result).toEqual({ count: 2, done: 1, percent: 75, running: true, saved: 60 });
  });

  it('treats a failure as finished', () => {
    const result = totals([item({ status: 'failed' })]);
    expect(result.running).toBe(false);
    expect(result.percent).toBe(100);
  });

  it('never counts a larger result as saved', () => {
    expect(totals([item({ status: 'done', outputSize: 500 })]).saved).toBe(0);
  });

  it('forgets the files finished in an earlier batch', () => {
    const result = totals([
      item({ status: 'done', progress: 100, outputSize: 40, run: 1 }),
      item({ status: 'done', progress: 100, outputSize: 40, run: 1 }),
      item({ status: 'waiting', run: 2 }),
    ]);
    expect(result).toEqual({ count: 1, done: 0, percent: 0, running: true, saved: 0 });
  });

  it('starts empty', () => {
    expect(totals([]).percent).toBe(0);
  });
});

describe('minutesLeft', () => {
  it('follows the pace so far', () => {
    expect(minutesLeft(60_000, 50)).toBe(1);
    expect(minutesLeft(120_000, 25)).toBe(6);
  });

  it('says nothing too early or at the end', () => {
    expect(minutesLeft(1000, 1)).toBeNull();
    expect(minutesLeft(1000, 100)).toBeNull();
  });
});
