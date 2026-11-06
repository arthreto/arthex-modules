/** A run of text with its inline style. Rendered as text nodes, never as HTML. */
export interface Span {
  text: string;
  bold?: boolean;
  italic?: boolean;
  strike?: boolean;
  code?: boolean;
  mark?: boolean;
  href?: string;
}

export type Tone = 'note' | 'tip' | 'warning';

export type Block =
  | { kind: 'h1' | 'h2' | 'h3' | 'p' | 'quote'; spans: Span[]; line: number }
  | { kind: 'li'; spans: Span[]; depth: number; line: number }
  | { kind: 'ol'; number: string; spans: Span[]; depth: number; line: number }
  | { kind: 'task'; done: boolean; spans: Span[]; line: number }
  | { kind: 'callout'; tone: Tone; spans: Span[]; line: number }
  | { kind: 'code'; language: string; code: string; line: number }
  | { kind: 'math'; tex: string; line: number }
  | { kind: 'table'; head: Span[][]; rows: Span[][][]; line: number }
  | { kind: 'image'; caption: string; file: string; line: number }
  | { kind: 'video'; caption: string; file: string; line: number }
  | { kind: 'file'; name: string; file: string; line: number }
  | { kind: 'details'; summary: Span[]; body: Block[]; line: number }
  | { kind: 'hr'; line: number }
  | { kind: 'gap'; line: number };

const INLINE =
  /(\*\*[^*]+\*\*|__[^_]+__|~~[^~]+~~|==[^=]+==|`[^`]+`|!?\[[^\]]+\]\([^)\s]+\)|\*[^*\s][^*]*\*|(?<!\w)_[^_\s][^_]*_(?!\w))/g;
const LINK = /^!?\[([^\]]+)\]\(([^)\s]+)\)$/;
const TASK = /^\s*[-*]\s+\[( |x|X)\]\s*(.*)$/;
const IMAGE = /^!\[([^\]]*)\]\(media\/([^)\s]+\.(?:png|jpe?g|gif|webp))\)$/i;
const VIDEO = /^!\[([^\]]*)\]\(media\/([^)\s]+\.(?:mp4|m4v|mov|webm))\)$/i;
const FILE = /^\[([^\]]+)\]\(media\/([^)\s]+)\)$/;
const CALLOUT = /^>\s*\[!(NOTE|TIP|WARNING)\]\s*(.*)$/;
const QUOTED = /^>\s?/;
const TABLE_RULE = /^\s*\|[\s:|-]+\|\s*$/;
const plainQuote = (line: string): boolean => QUOTED.test(line) && !CALLOUT.test(line);

export function inline(source: string): Span[] {
  const spans: Span[] = [];
  let last = 0;
  for (const match of source.matchAll(INLINE)) {
    const token = match[0];
    if (match.index > last) spans.push({ text: source.slice(last, match.index) });
    const link = LINK.exec(token);
    if (link) spans.push({ text: link[1] ?? '', href: link[2] ?? '' });
    else if (token.startsWith('**') || token.startsWith('__'))
      spans.push({ text: token.slice(2, -2), bold: true });
    else if (token.startsWith('~~')) spans.push({ text: token.slice(2, -2), strike: true });
    else if (token.startsWith('==')) spans.push({ text: token.slice(2, -2), mark: true });
    else if (token.startsWith('`')) spans.push({ text: token.slice(1, -1), code: true });
    else spans.push({ text: token.slice(1, -1), italic: true });
    last = match.index + token.length;
  }
  if (last < source.length) spans.push({ text: source.slice(last) });
  return spans;
}

const cellsOf = (row: string): string[] =>
  row
    .trim()
    .replace(/^\||\|$/g, '')
    .split('|')
    .map((cell) => cell.trim());

const depthOf = (indent: string): number => Math.floor(indent.length / 2);

function parseLines(lines: string[], offset: number): Block[] {
  const blocks: Block[] = [];
  /** The next lines as long as `keep` holds, moving past them. */
  const takeWhile = (from: number, keep: (line: string) => boolean): [string[], number] => {
    const taken: string[] = [];
    let at = from;
    while (at < lines.length && keep(lines[at] ?? '')) {
      taken.push(lines[at] ?? '');
      at += 1;
    }
    return [taken, at];
  };

  for (let index = 0; index < lines.length; index += 1) {
    const content = (lines[index] ?? '').trimEnd();
    const line = index + offset;
    let match: RegExpExecArray | null;
    if (!content.trim()) {
      if (blocks.length > 0 && blocks.at(-1)?.kind !== 'gap') blocks.push({ kind: 'gap', line });
    } else if (/^\s*```/.test(content)) {
      const [code, end] = takeWhile(index + 1, (next) => !/^\s*```/.test(next));
      blocks.push({ kind: 'code', language: content.trim().slice(3), code: code.join('\n'), line });
      index = end;
    } else if (content.trim() === '$$') {
      const [tex, end] = takeWhile(index + 1, (next) => next.trim() !== '$$');
      blocks.push({ kind: 'math', tex: tex.join('\n').trim(), line });
      index = end;
    } else if (/^\s*<details>/.test(content)) {
      const [body, end] = takeWhile(index + 1, (next) => !/^\s*<\/details>/.test(next));
      const summary = /<summary>(.*)<\/summary>/.exec(content)?.[1] ?? '';
      blocks.push({ kind: 'details', summary: inline(summary), body: parseLines(body, line + 1), line });
      index = end;
    } else if (/^\s*\|/.test(content) && TABLE_RULE.test(lines[index + 1] ?? '')) {
      const [rows, end] = takeWhile(index + 2, (next) => /^\s*\|/.test(next));
      blocks.push({
        kind: 'table',
        head: cellsOf(content).map(inline),
        rows: rows.map((row) => cellsOf(row).map(inline)),
        line,
      });
      index = end - 1;
    } else if (/^\s*(---+|\*\*\*+)\s*$/.test(content)) {
      blocks.push({ kind: 'hr', line });
    } else if ((match = IMAGE.exec(content))) {
      blocks.push({ kind: 'image', caption: match[1] ?? '', file: match[2] ?? '', line });
    } else if ((match = VIDEO.exec(content))) {
      blocks.push({ kind: 'video', caption: match[1] ?? '', file: match[2] ?? '', line });
    } else if ((match = FILE.exec(content))) {
      blocks.push({ kind: 'file', name: match[1] ?? '', file: match[2] ?? '', line });
    } else if ((match = CALLOUT.exec(content))) {
      const [rest, end] = takeWhile(index + 1, plainQuote);
      const text = [match[2] ?? '', ...rest.map((next) => next.replace(QUOTED, ''))].join(' ').trim();
      const tone = (match[1] ?? 'NOTE').toLowerCase() as Tone;
      blocks.push({ kind: 'callout', tone, spans: inline(text), line });
      index = end - 1;
    } else if (QUOTED.test(content)) {
      const [rest, end] = takeWhile(index, plainQuote);
      const text = rest.map((next) => next.replace(QUOTED, '').trim()).join(' ');
      blocks.push({ kind: 'quote', spans: inline(text), line });
      index = end - 1;
    } else if ((match = /^(#{1,3})\s+(.*)$/.exec(content))) {
      const level = match[1]?.length ?? 1;
      const kind = level === 1 ? 'h1' : level === 2 ? 'h2' : 'h3';
      blocks.push({ kind, spans: inline(match[2] ?? ''), line });
    } else if ((match = TASK.exec(content))) {
      blocks.push({ kind: 'task', done: match[1] !== ' ', spans: inline(match[2] ?? ''), line });
    } else if ((match = /^(\s*)[-*]\s+(.*)$/.exec(content))) {
      blocks.push({ kind: 'li', spans: inline(match[2] ?? ''), depth: depthOf(match[1] ?? ''), line });
    } else if ((match = /^(\s*)(\d+)[.)]\s+(.*)$/.exec(content))) {
      blocks.push({
        kind: 'ol',
        number: match[2] ?? '1',
        spans: inline(match[3] ?? ''),
        depth: depthOf(match[1] ?? ''),
        line,
      });
    } else {
      blocks.push({ kind: 'p', spans: inline(content), line });
    }
  }
  return blocks;
}

/** Line by line, each block knowing the line it starts on, for what changes the source. */
export function parse(text: string): Block[] {
  return parseLines(text.split('\n'), 0);
}

/** A line without its Markdown marks, for previews. */
export function plain(line: string): string {
  return line
    .replace(/^#{1,3}\s+/, '')
    .replace(/^>\s*(\[![A-Z]+\]\s*)?/, '')
    .replace(/^\s*[-*]\s+(\[[ xX]\]\s*)?/, '')
    .replace(/^\s*\d+[.)]\s+/, '')
    .replace(/\[([^\]]+)\]\([^)\s]+\)/g, '$1')
    .replace(/\*\*|__|~~|==|`/g, '')
    .replace(/(^|\W)[*_]([^*_]+)[*_](?=\W|$)/g, '$1$2');
}

/** The first line that reads as text, for the list of documents. */
export function firstLine(text: string): string {
  const line = text
    .split('\n')
    .find((candidate) => candidate.trim() && !/^\s*(\$\$|```|---|\*\*\*|\||!\[|<)/.test(candidate));
  return line ? plain(line) : '';
}

export function taskCount(text: string): { done: number; total: number } {
  const found = text.split('\n').flatMap((line) => {
    const match = TASK.exec(line);
    return match ? [match[1] !== ' '] : [];
  });
  return { done: found.filter(Boolean).length, total: found.length };
}

/** Ticks or unticks the task on a line of the source. */
export function toggleTask(text: string, line: number): string {
  const lines = text.split('\n');
  const current = lines[line];
  if (current === undefined) return text;
  lines[line] = current.replace(/\[( |x|X)\]/, (_, mark: string) => (mark === ' ' ? '[x]' : '[ ]'));
  return lines.join('\n');
}

/** The result of an edit: the new text and the selection to restore. */
export interface Edit {
  text: string;
  start: number;
  end: number;
}

/** Surrounds the selection with a mark, or a placeholder when nothing is selected. */
export function wrap(
  text: string,
  start: number,
  end: number,
  mark: string,
  placeholder: string,
  closing = mark,
): Edit {
  const selected = text.slice(start, end) || placeholder;
  return {
    text: text.slice(0, start) + mark + selected + closing + text.slice(end),
    start: start + mark.length,
    end: start + mark.length + selected.length,
  };
}

const PREFIX = /^(#{1,3}\s+|>\s+|\s*[-*]\s+\[[ xX]\]\s+|\s*[-*]\s+|\s*\d+[.)]\s+)/;

/** Gives the line under the caret a prefix (`## `, `- `), replaces another one, or removes it when
 * it is already there. */
export function togglePrefix(text: string, start: number, end: number, prefix: string): Edit {
  const lineStart = text.lastIndexOf('\n', start - 1) + 1;
  const found = text.indexOf('\n', start);
  const lineEnd = found < 0 ? text.length : found;
  const line = text.slice(lineStart, lineEnd);
  const current = PREFIX.exec(line)?.[0];
  const next =
    current === prefix ? line.slice(prefix.length) : prefix + (current ? line.slice(current.length) : line);
  const delta = next.length - line.length;
  return {
    text: text.slice(0, lineStart) + next + text.slice(lineEnd),
    start: Math.max(lineStart, start + delta),
    end: Math.max(lineStart, end + delta),
  };
}

/** Enter in a list: continues it with a new item, or ends it when the item is empty. `null` when
 * the caret is not in a list. */
export function continueList(text: string, caret: number): Edit | null {
  const lineStart = text.lastIndexOf('\n', caret - 1) + 1;
  const line = text.slice(lineStart, caret);
  const match = /^(\s*)([-*]\s+\[[ xX]\]\s+|[-*]\s+|(\d+)[.)]\s+)/.exec(line);
  if (!match) return null;
  if (!line.slice(match[0].length).trim()) {
    return { text: text.slice(0, lineStart) + text.slice(caret), start: lineStart, end: lineStart };
  }
  const marker = match[3] ? `${Number(match[3]) + 1}. ` : (match[2] ?? '').replace(/\[[xX]\]/, '[ ]');
  const insert = `\n${match[1] ?? ''}${marker}`;
  const position = caret + insert.length;
  return { text: text.slice(0, caret) + insert + text.slice(caret), start: position, end: position };
}

/** What follows `/` when the caret sits on a line that starts with it, or `null`. */
export function slashQuery(text: string, caret: number): string | null {
  const lineStart = text.lastIndexOf('\n', caret - 1) + 1;
  const match = /^\/([\p{L} ]*)$/u.exec(text.slice(lineStart, caret));
  return match ? (match[1] ?? '') : null;
}

/** Puts a block in place of the selection, on a line of its own, the caret at `caretAt` in it or
 * after it. With `slash`, it also takes the place of the `/…` being typed. */
export function insertBlock(
  text: string,
  start: number,
  end: number,
  block: string,
  { slash = false, caretAt = block.length }: { slash?: boolean; caretAt?: number } = {},
): Edit {
  const from = slash ? text.lastIndexOf('/', start - 1) : start;
  const before = text.slice(0, from);
  const lead = before && !before.endsWith('\n') ? '\n' : '';
  const position = before.length + lead.length + caretAt;
  return { text: before + lead + block + text.slice(end), start: position, end: position };
}
