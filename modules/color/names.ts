import en from './locales/en.json';
import fr from './locales/fr.json';

export const NAMES = [
  { id: 'red', hex: '#FF0000' },
  { id: 'cherry', hex: '#DE3163' },
  { id: 'brick', hex: '#B22222' },
  { id: 'burgundy', hex: '#800020' },
  { id: 'coral', hex: '#FF7F50' },
  { id: 'orange', hex: '#FFA500' },
  { id: 'darkOrange', hex: '#FF8C00' },
  { id: 'yellow', hex: '#FFFF00' },
  { id: 'gold', hex: '#FFD700' },
  { id: 'green', hex: '#008000' },
  { id: 'lime', hex: '#32CD32' },
  { id: 'mint', hex: '#98FF98' },
  { id: 'forest', hex: '#228B22' },
  { id: 'turquoise', hex: '#40E0D0' },
  { id: 'cyan', hex: '#00FFFF' },
  { id: 'blue', hex: '#0000FF' },
  { id: 'skyBlue', hex: '#87CEEB' },
  { id: 'royalBlue', hex: '#4169E1' },
  { id: 'navy', hex: '#000080' },
  { id: 'steelBlue', hex: '#4682B4' },
  { id: 'indigo', hex: '#4B0082' },
  { id: 'purple', hex: '#8A2BE2' },
  { id: 'plum', hex: '#DDA0DD' },
  { id: 'lavender', hex: '#E6E6FA' },
  { id: 'pink', hex: '#FFC0CB' },
  { id: 'hotPink', hex: '#FF69B4' },
  { id: 'magenta', hex: '#FF00FF' },
  { id: 'brown', hex: '#8B4513' },
  { id: 'beige', hex: '#F5F5DC' },
  { id: 'white', hex: '#FFFFFF' },
  { id: 'gray', hex: '#808080' },
  { id: 'lightGray', hex: '#D3D3D3' },
  { id: 'charcoal', hex: '#36454F' },
  { id: 'black', hex: '#000000' },
] as const;

export type NamedColor = (typeof NAMES)[number];

/** Short words like "or" are too often the start of something else: below this, only exact names count. */
const MIN_PREFIX = 3;

const fold = (text: string) =>
  text
    .normalize('NFD')
    .replace(/\p{Diacritic}/gu, '')
    .toLowerCase();

const writings = (color: NamedColor) => [fold(en.names[color.id]), fold(fr.names[color.id])];

/** The colours a search names, in either language: exact names first, then the words they start. */
export function matchNames(query: string): NamedColor[] {
  const wanted = fold(query.trim());
  if (!wanted) return [];
  const exact = NAMES.filter((color) => writings(color).includes(wanted));
  if (wanted.length < MIN_PREFIX) return exact;
  const starting = NAMES.filter(
    (color) =>
      !exact.some((other) => other.id === color.id) &&
      writings(color).some((name) => name.split(' ').some((word) => word.startsWith(wanted))),
  );
  return [...exact, ...starting];
}

export const exactName = (query: string): NamedColor | undefined => {
  const wanted = fold(query.trim());
  return NAMES.find((color) => writings(color).includes(wanted));
};
