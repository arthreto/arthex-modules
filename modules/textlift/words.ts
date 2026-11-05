/** A word found in an image, in the image's pixels. */
export interface Word {
  text: string;
  x: number;
  y: number;
  w: number;
  h: number;
  line: number;
}

export interface Point {
  x: number;
  y: number;
}

export interface Zone {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Below this, in the image's pixels, it was a click and not a zone. */
const SMALLEST_ZONE = 6;
const MOST_ENLARGED = 1.6;

export const zoneBetween = (from: Point, to: Point): Zone => ({
  x: Math.min(from.x, to.x),
  y: Math.min(from.y, to.y),
  w: Math.abs(from.x - to.x),
  h: Math.abs(from.y - to.y),
});

export const isZone = (zone: Zone): boolean => zone.w >= SMALLEST_ZONE && zone.h >= SMALLEST_ZONE;

/** A word belongs to a zone when its centre does, so that a zone need not be drawn exactly. */
export function isInZone(word: Word, zone: Zone | null): boolean {
  if (!zone) return true;
  const x = word.x + word.w / 2;
  const y = word.y + word.h / 2;
  return x >= zone.x && x <= zone.x + zone.w && y >= zone.y && y <= zone.y + zone.h;
}

/** The words back as text, one line of the image per line. */
export function textOf(words: readonly Word[]): string {
  return words.reduce((text, word, index) => {
    if (index === 0) return word.text;
    return text + (words[index - 1]?.line === word.line ? ' ' : '\n') + word.text;
  }, '');
}

/** How large an image shows in the room there is: small ones grow a little, large ones fit. */
export function fitted(width: number, height: number, roomWidth: number, roomHeight: number) {
  const scale = Math.min(roomWidth / width, roomHeight / height, MOST_ENLARGED);
  return { width: width * scale, height: height * scale };
}
