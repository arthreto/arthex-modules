/** What the `/` menu and the Insert button offer. */

export type BlockGroup = 'text' | 'blocks' | 'media';
export type BlockId =
  | 'h1'
  | 'h2'
  | 'h3'
  | 'list'
  | 'numbered'
  | 'task'
  | 'quote'
  | 'callout'
  | 'details'
  | 'code'
  | 'math'
  | 'table'
  | 'separator'
  | 'image'
  | 'video'
  | 'file';

export interface BlockItem {
  id: BlockId;
  group: BlockGroup;
  icon: string;
}

const LIST = 'M9 6h11M9 12h11M9 18h11M4.5 6h.01M4.5 12h.01M4.5 18h.01';
const TASK = 'M5 4h14a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1ZM8 12l3 3 5-6';
const QUOTE = 'M7 8h3v6a3 3 0 0 1-3 3M15 8h3v6a3 3 0 0 1-3 3';
const CODE = 'm8 8-4 4 4 4M16 8l4 4-4 4M14 5l-4 14';

export const ICONS = {
  bold: 'M7 5h6a3.5 3.5 0 0 1 0 7H7ZM7 12h7a3.5 3.5 0 0 1 0 7H7Z',
  italic: 'M10 5h7M7 19h7M14 5l-4 14',
  strike: 'M5 12h14M16 7a4 3 0 0 0-4-2c-3 0-4 2-4 3.5M8 17a4 3 0 0 0 4 2c3 0 4-1.5 4-3.5',
  mark: 'm5 19 3-1 9-9-2-2-9 9ZM14 8l2 2M4 22h16',
  code: 'm8 8-4 4 4 4M16 8l4 4-4 4',
  link: 'M10 14a4 4 0 0 0 6 0l3-3a4 4 0 0 0-6-6l-1 1M14 10a4 4 0 0 0-6 0l-3 3a4 4 0 0 0 6 6l1-1',
  heading: 'M6 5v14M18 5v14M6 12h12',
  list: LIST,
  task: TASK,
  quote: QUOTE,
  plus: 'M12 5v14M5 12h14',
  folderPlus: 'M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2ZM12 11v5M9.5 13.5h5',
  chevron: 'm9 6 6 6-6 6',
  pencil: 'M4 20h4L19 9l-4-4L4 16ZM13 7l4 4',
  file: 'M7 3h7l5 5v12a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1ZM14 3v5h5',
  copy: 'M9 9h10a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1V10a1 1 0 0 1 1-1ZM16 9V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3',
  note: 'M12 8h.01M11 12h1v4h1M4 12a8 8 0 1 0 16 0 8 8 0 0 0-16 0Z',
  tip: 'M9 18h6M10 21h4M12 3a6 6 0 0 0-4 10.5c.7.7 1 1.5 1 2.5h6c0-1 .3-1.8 1-2.5A6 6 0 0 0 12 3Z',
  warning: 'M12 4 3 20h18ZM12 10v4M12 17h.01',
};

export const BLOCKS: readonly BlockItem[] = [
  { id: 'h1', group: 'text', icon: 'M4 6v12M12 6v12M4 12h8M17 18v-8l-2 1.5' },
  { id: 'h2', group: 'text', icon: 'M4 6v12M12 6v12M4 12h8M16 11a2 2 0 1 1 3.5 1.3L16 18h4' },
  { id: 'h3', group: 'text', icon: 'M4 6v12M12 6v12M4 12h8M16 10h3l-2 3a2.5 2.5 0 1 1-2 4' },
  { id: 'list', group: 'text', icon: LIST },
  {
    id: 'numbered',
    group: 'text',
    icon: 'M10 6h10M10 12h10M10 18h10M4 5l1.5-1v5M4 15a1.5 1.5 0 1 1 2.5 1L4 19h3',
  },
  { id: 'task', group: 'text', icon: TASK },
  { id: 'quote', group: 'text', icon: QUOTE },
  { id: 'callout', group: 'blocks', icon: ICONS.note },
  { id: 'details', group: 'blocks', icon: 'm8 9 4 4 4-4M4 5h16M4 19h16' },
  { id: 'code', group: 'blocks', icon: CODE },
  { id: 'math', group: 'blocks', icon: 'M18 5H6l7 7-7 7h12' },
  {
    id: 'table',
    group: 'blocks',
    icon: 'M4 5h16a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1ZM3 10h18M3 15h18M9 5v14M15 5v14',
  },
  { id: 'separator', group: 'blocks', icon: 'M4 12h16' },
  {
    id: 'image',
    group: 'media',
    icon: 'M5 4h14a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1ZM4 16l5-5 4 4 2-2 5 5M15 8.5h.01',
  },
  {
    id: 'video',
    group: 'media',
    icon: 'M4 6h11a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1ZM16 11l5-3v8l-5-3',
  },
  { id: 'file', group: 'media', icon: ICONS.file },
];

/** The blocks with a word of their name starting with what was typed after `/`. */
export function matchingBlocks(query: string, label: (id: BlockId) => string): BlockItem[] {
  const wanted = query.trim().toLocaleLowerCase();
  return BLOCKS.filter((item) =>
    label(item.id)
      .toLocaleLowerCase()
      .split(' ')
      .some((word) => word.startsWith(wanted)),
  );
}

/** The next one in a list that goes around. */
export const moved = (index: number, by: number, length: number): number =>
  length === 0 ? 0 : (index + by + length) % length;
