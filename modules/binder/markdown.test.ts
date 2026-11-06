import { describe, expect, it } from 'vitest';
import {
  continueList,
  firstLine,
  inline,
  insertBlock,
  parse,
  plain,
  slashQuery,
  taskCount,
  toggleTask,
  togglePrefix,
  wrap,
} from './markdown';

const kinds = (text: string) => parse(text).map((block) => block.kind);

describe('inline', () => {
  it('reads every style, keeping the plain text around them', () => {
    expect(inline('a **b** c')).toEqual([{ text: 'a ' }, { text: 'b', bold: true }, { text: ' c' }]);
    expect(inline('*i*')).toEqual([{ text: 'i', italic: true }]);
    expect(inline('~~s~~')).toEqual([{ text: 's', strike: true }]);
    expect(inline('==m==')).toEqual([{ text: 'm', mark: true }]);
    expect(inline('`c`')).toEqual([{ text: 'c', code: true }]);
  });

  it('reads a link, and never makes HTML of what it reads', () => {
    expect(inline('[site](https://a.fr)')).toEqual([{ text: 'site', href: 'https://a.fr' }]);
    expect(inline('<img onerror=x>')).toEqual([{ text: '<img onerror=x>' }]);
  });
});

describe('parse', () => {
  it('reads titles, lists and tasks, with the line each one is on', () => {
    expect(parse('# A\n\n- b\n  - c\n1. d\n- [x] e')).toEqual([
      { kind: 'h1', spans: [{ text: 'A' }], line: 0 },
      { kind: 'gap', line: 1 },
      { kind: 'li', spans: [{ text: 'b' }], depth: 0, line: 2 },
      { kind: 'li', spans: [{ text: 'c' }], depth: 1, line: 3 },
      { kind: 'ol', number: '1', spans: [{ text: 'd' }], depth: 0, line: 4 },
      { kind: 'task', done: true, spans: [{ text: 'e' }], line: 5 },
    ]);
  });

  it('reads a separator, a quote and every kind of callout', () => {
    expect(kinds('---\n> a\n> b\n> [!WARNING] c\n> d')).toEqual(['hr', 'quote', 'callout']);
    const [quote, callout] = parse('> a\n> b\n\n> [!TIP] c\n> d').filter((block) => block.kind !== 'gap');
    expect(quote).toMatchObject({ spans: [{ text: 'a b' }] });
    expect(callout).toMatchObject({ tone: 'tip', spans: [{ text: 'c d' }] });
  });

  it('keeps code and formulas as they are written, even unfinished', () => {
    expect(parse('```python\na = **1**\n```\nb')).toEqual([
      { kind: 'code', language: 'python', code: 'a = **1**', line: 0 },
      { kind: 'p', spans: [{ text: 'b' }], line: 3 },
    ]);
    expect(parse('$$\nE = mc^2\n$$')).toEqual([{ kind: 'math', tex: 'E = mc^2', line: 0 }]);
    expect(parse('```\nsans fin')).toEqual([{ kind: 'code', language: '', code: 'sans fin', line: 0 }]);
  });

  it('reads a table, and only with its rule under the first row', () => {
    const [table] = parse('| A | B |\n| --- | :-: |\n| 1 | **2** |\nfin');
    expect(table).toMatchObject({
      kind: 'table',
      head: [[{ text: 'A' }], [{ text: 'B' }]],
      rows: [[[{ text: '1' }], [{ text: '2', bold: true }]]],
    });
    expect(kinds('| A | B |\n| --- | --- |\n| 1 | 2 |\nfin')).toEqual(['table', 'p']);
    expect(kinds('| pas | un tableau |')).toEqual(['p']);
  });

  it('shows only what was copied in, by its kind', () => {
    expect(parse('![Légende](media/a.png)')).toEqual([
      { kind: 'image', caption: 'Légende', file: 'a.png', line: 0 },
    ]);
    expect(parse('![Cours](media/a.MP4)')).toEqual([
      { kind: 'video', caption: 'Cours', file: 'a.MP4', line: 0 },
    ]);
    expect(parse('[Poly.pdf](media/a.pdf)')).toEqual([
      { kind: 'file', name: 'Poly.pdf', file: 'a.pdf', line: 0 },
    ]);
  });

  it('never loads anything from elsewhere', () => {
    expect(parse('![x](https://a.fr/b.png)')).toEqual([
      { kind: 'p', spans: [{ text: 'x', href: 'https://a.fr/b.png' }], line: 0 },
    ]);
    expect(kinds('![x](C:/a.png)')).toEqual(['p']);
    expect(kinds('![x](media/../a.exe)')).toEqual(['p']);
  });

  it('reads a folded section with blocks inside, whose lines still point to the source', () => {
    const [details] = parse('<details><summary>Q **a**</summary>\n- [ ] b\n</details>');
    expect(details).toMatchObject({
      kind: 'details',
      summary: [{ text: 'Q ' }, { text: 'a', bold: true }],
      body: [{ kind: 'task', done: false, line: 1 }],
    });
  });

  it('goes on after each block that spans several lines', () => {
    expect(kinds('```\nx\n```\n$$\ny\n$$\n<details><summary>z</summary>\nw\n</details>\nfin')).toEqual([
      'code',
      'math',
      'details',
      'p',
    ]);
  });
});

describe('the list of documents', () => {
  it('reads a line without its marks', () => {
    expect(plain('## **Un** `titre`')).toBe('Un titre');
    expect(plain('> [!NOTE] À revoir')).toBe('À revoir');
    expect(plain('- [x] fait')).toBe('fait');
    expect(plain('voir [le poly](media/a.pdf)')).toBe('voir le poly');
  });

  it('skips what does not read as text', () => {
    expect(firstLine('$$\n```\n---\n| a |\n![x](media/a.png)\nEnfin du texte')).toBe('Enfin du texte');
    expect(firstLine('   \n')).toBe('');
  });

  it('counts the tasks', () => {
    expect(taskCount('- [x] a\n- [ ] b\ntexte')).toEqual({ done: 1, total: 2 });
  });
});

describe('editing', () => {
  it('ticks the task of a line only', () => {
    expect(toggleTask('- [ ] a\n- [x] b', 1)).toBe('- [ ] a\n- [ ] b');
    expect(toggleTask('- [ ] a', 5)).toBe('- [ ] a');
  });

  it('surrounds the selection or a placeholder', () => {
    expect(wrap('abc', 1, 2, '**', 'x')).toEqual({ text: 'a**b**c', start: 3, end: 4 });
    expect(wrap('', 0, 0, '==', 'x')).toEqual({ text: '==x==', start: 2, end: 3 });
    expect(wrap('', 0, 0, '[', 'x', '](https://)').text).toBe('[x](https://)');
  });

  it('gives a line a prefix, replaces one, removes it', () => {
    expect(togglePrefix('a', 0, 0, '> ').text).toBe('> a');
    expect(togglePrefix('- a', 0, 0, '> ').text).toBe('> a');
    expect(togglePrefix('> a', 2, 2, '> ').text).toBe('a');
  });

  it('continues a list, and ends it on an empty item', () => {
    expect(continueList('1. a', 4)?.text).toBe('1. a\n2. ');
    expect(continueList('- [x] a', 7)?.text).toBe('- [x] a\n- [ ] ');
    expect(continueList('- ', 2)?.text).toBe('');
    expect(continueList('a', 1)).toBeNull();
  });

  it('notices a slash only at the start of a line', () => {
    expect(slashQuery('/', 1)).toBe('');
    expect(slashQuery('a\n/tab', 6)).toBe('tab');
    expect(slashQuery('/liste num', 10)).toBe('liste num');
    expect(slashQuery('a/b', 3)).toBeNull();
    expect(slashQuery('/a/b', 4)).toBeNull();
    expect(slashQuery('1/2', 3)).toBeNull();
  });

  it('puts a block on its own line, in place of the slash', () => {
    expect(insertBlock('ab', 2, 2, '---')).toEqual({ text: 'ab\n---', start: 6, end: 6 });
    expect(insertBlock('', 0, 0, '---')).toEqual({ text: '---', start: 3, end: 3 });
    expect(insertBlock('a\n/tab', 6, 6, '---', { slash: true })).toEqual({
      text: 'a\n---',
      start: 5,
      end: 5,
    });
    expect(insertBlock('', 0, 0, '$$\n\n$$', { caretAt: 3 })).toMatchObject({ start: 3 });
  });
});
