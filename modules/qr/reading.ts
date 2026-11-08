/** Larger images are reduced first: a code stays readable and the reading stays quick. */
const LARGEST_SIDE = 1600;

export interface Point {
  x: number;
  y: number;
}

export interface Shot {
  /** A JPEG copy to show, in a `data:` address the window may load. */
  src: string;
  width: number;
  height: number;
}

export interface Found {
  content: string;
  version: number;
  bytes: number;
  /** The corners of the code in the shot, clockwise from the top left. */
  corners: Point[];
}

export interface Reading {
  shot: Shot;
  found: Found | null;
}

/** An image from a file, a drop or a paste, or the `data:` address of a capture. */
async function bitmapOf(source: Blob | string): Promise<ImageBitmap> {
  if (typeof source !== 'string') return createImageBitmap(source);
  const image = new Image();
  image.src = source;
  await image.decode();
  return createImageBitmap(image);
}

export async function read(source: Blob | string): Promise<Reading> {
  const [bitmap, { default: jsQR }] = await Promise.all([bitmapOf(source), import('jsqr')]);
  const scale = Math.min(1, LARGEST_SIDE / Math.max(bitmap.width, bitmap.height));
  const width = Math.max(1, Math.round(bitmap.width * scale));
  const height = Math.max(1, Math.round(bitmap.height * scale));
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (!context) throw new Error('no 2d context');
  context.drawImage(bitmap, 0, 0, width, height);
  bitmap.close();
  const code = jsQR(context.getImageData(0, 0, width, height).data, width, height, {
    inversionAttempts: 'attemptBoth',
  });
  const shot = { src: canvas.toDataURL('image/jpeg', 0.86), width, height };
  if (!code) return { shot, found: null };
  const { topLeftCorner, topRightCorner, bottomRightCorner, bottomLeftCorner } = code.location;
  return {
    shot,
    found: {
      content: code.data,
      version: code.version,
      bytes: code.binaryData.length,
      corners: [topLeftCorner, topRightCorner, bottomRightCorner, bottomLeftCorner],
    },
  };
}

/** The first image of a drop or a paste. */
export const imageIn = (files: FileList | null | undefined): File | null =>
  [...(files ?? [])].find((file) => file.type.startsWith('image/')) ?? null;
