/** Red, green and blue from 0 to 255, and alpha from 0 to 1. */
export interface Color {
  r: number;
  g: number;
  b: number;
  a: number;
}

/** Hue in degrees, saturation and value from 0 to 1. */
export interface Hsv {
  h: number;
  s: number;
  v: number;
}

export type Format = 'HEX' | 'RGB' | 'RGBA' | 'HSL' | 'HSLA' | 'HSV' | 'CMYK';

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));
const percent = (value: number) => Math.round(value * 100);
const pair = (value: number) => value.toString(16).padStart(2, '0');

export function toHsv({ r, g, b }: Color): Hsv {
  const [red, green, blue] = [r / 255, g / 255, b / 255];
  const max = Math.max(red, green, blue);
  const delta = max - Math.min(red, green, blue);
  let hue = 0;
  if (delta !== 0) {
    if (max === red) hue = ((green - blue) / delta + 6) % 6;
    else if (max === green) hue = (blue - red) / delta + 2;
    else hue = (red - green) / delta + 4;
  }
  return { h: hue * 60, s: max === 0 ? 0 : delta / max, v: max };
}

export function fromHsv({ h, s, v }: Hsv, a = 1): Color {
  const channel = (offset: number) => {
    const k = (offset + h / 60) % 6;
    return Math.round((v - v * s * clamp(Math.min(k, 4 - k), 0, 1)) * 255);
  };
  return { r: channel(5), g: channel(3), b: channel(1), a };
}

function fromHsl(h: number, s: number, l: number, a: number): Color {
  const value = l + s * Math.min(l, 1 - l);
  return fromHsv({ h, s: value === 0 ? 0 : 2 * (1 - l / value), v: value }, a);
}

function fromCmyk(c: number, m: number, y: number, k: number): Color {
  const channel = (ink: number) => Math.round(255 * (1 - ink) * (1 - k));
  return { r: channel(c), g: channel(m), b: channel(y), a: 1 };
}

/** `#RRGGBB`, or `#RRGGBBAA` when the colour is not opaque. */
export function toHex({ r, g, b, a }: Color): string {
  const alpha = a < 1 ? pair(Math.round(a * 255)) : '';
  return `#${pair(r)}${pair(g)}${pair(b)}${alpha}`.toUpperCase();
}

function fromHex(digits: string): Color {
  const full = digits.length < 6 ? [...digits].map((digit) => digit + digit).join('') : digits;
  const part = (start: number) => parseInt(full.slice(start, start + 2), 16);
  return { r: part(0), g: part(2), b: part(4), a: full.length === 8 ? part(6) / 255 : 1 };
}

/** The numbers of "rgb(76, 194, 255)" or "76 194 255 / 50%": a percent stays out of 100. */
function numbers(text: string): number[] | null {
  const parts = text.split(/[\s,/]+/).filter(Boolean);
  const values = parts.map((part) => parseFloat(part));
  return values.every(Number.isFinite) ? values : null;
}

function alphaOf(text: string, value: number | undefined): number {
  if (value === undefined) return 1;
  return clamp(/\d%\s*$/.test(text) ? value / 100 : value, 0, 1);
}

const isByte = (value: number) => value >= 0 && value <= 255;

function fromFunction(name: string, inner: string): Color | null {
  const values = numbers(inner);
  const [first, second, third, fourth] = values ?? [];
  if (first === undefined || second === undefined || third === undefined) return null;
  const alpha = alphaOf(inner, fourth);
  switch (name) {
    case 'rgb':
    case 'rgba':
      if (![first, second, third].every(isByte)) return null;
      return { r: Math.round(first), g: Math.round(second), b: Math.round(third), a: alpha };
    case 'hsl':
    case 'hsla':
      return fromHsl(((first % 360) + 360) % 360, clamp(second / 100, 0, 1), clamp(third / 100, 0, 1), alpha);
    case 'hsv':
    case 'hsb':
      return fromHsv(
        { h: ((first % 360) + 360) % 360, s: clamp(second / 100, 0, 1), v: clamp(third / 100, 0, 1) },
        alpha,
      );
    case 'cmyk':
      if (fourth === undefined) return null;
      return fromCmyk(
        ...([first, second, third, fourth].map((ink) => clamp(ink / 100, 0, 1)) as [
          number,
          number,
          number,
          number,
        ]),
      );
    default:
      return null;
  }
}

/**
 * A colour as people write it. A code without `#` must look like one, mixing digits and letters
 * on 6 or 8 characters: "facade" is a word and "123456" a number, not colours.
 */
export function parseColor(text: string): Color | null {
  const trimmed = text.trim();
  const hash = /^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i.exec(trimmed);
  if (hash?.[1]) return fromHex(hash[1]);
  if (/^(?=.*\d)(?=.*[a-f])(?:[0-9a-f]{6}|[0-9a-f]{8})$/i.test(trimmed)) return fromHex(trimmed);
  const called = /^([a-z]+)\(([^)]*)\)$/i.exec(trimmed);
  if (called?.[1] !== undefined && called[2] !== undefined)
    return fromFunction(called[1].toLowerCase(), called[2]);
  const triple = /^(\d{1,3})[\s,]+(\d{1,3})[\s,]+(\d{1,3})$/.exec(trimmed);
  if (triple) {
    const channels = [Number(triple[1]), Number(triple[2]), Number(triple[3])];
    if (channels.every(isByte))
      return { r: channels[0] ?? 0, g: channels[1] ?? 0, b: channels[2] ?? 0, a: 1 };
  }
  return null;
}

/** The same colour in every format, in the order they are shown. */
export function formats(color: Color): { format: Format; value: string }[] {
  const { r, g, b, a } = color;
  const { h, s: hsvSaturation, v } = toHsv(color);
  const [red, green, blue] = [r / 255, g / 255, b / 255];
  const max = Math.max(red, green, blue);
  const min = Math.min(red, green, blue);
  const lightness = (max + min) / 2;
  const delta = max - min;
  const hslSaturation = delta === 0 ? 0 : delta / (1 - Math.abs(2 * lightness - 1));
  const hue = Math.round(h) % 360;
  const alpha = Math.round(a * 100) / 100;
  const hsl = `${hue}, ${percent(hslSaturation)}%, ${percent(lightness)}%`;
  const key = 1 - max;
  const ink = (channel: number) => (key === 1 ? 0 : percent((1 - channel - key) / (1 - key)));
  return [
    { format: 'HEX', value: toHex(color) },
    { format: 'RGB', value: `rgb(${r}, ${g}, ${b})` },
    { format: 'RGBA', value: `rgba(${r}, ${g}, ${b}, ${alpha})` },
    { format: 'HSL', value: `hsl(${hsl})` },
    { format: 'HSLA', value: `hsla(${hsl}, ${alpha})` },
    { format: 'HSV', value: `hsv(${hue}, ${percent(hsvSaturation)}%, ${percent(v)}%)` },
    { format: 'CMYK', value: `cmyk(${ink(red)}%, ${ink(green)}%, ${ink(blue)}%, ${percent(key)}%)` },
  ];
}
