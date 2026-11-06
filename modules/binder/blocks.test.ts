import { describe, expect, it } from 'vitest';
import { BLOCKS, matchingBlocks, moved, type BlockId } from './blocks';
import fr from './locales/fr.json';

const label = (id: BlockId) => fr.blocks[id];

describe('the menu of blocks', () => {
  it('offers everything before anything is typed', () => {
    expect(matchingBlocks('', label)).toHaveLength(BLOCKS.length);
  });

  it('matches the start of a word of the name, not its middle', () => {
    expect(matchingBlocks('ta', label).map((item) => item.id)).toEqual(['table']);
    expect(matchingBlocks('num', label).map((item) => item.id)).toEqual(['numbered']);
    expect(matchingBlocks('TITRE', label).map((item) => item.id)).toEqual(['h1', 'h2', 'h3']);
    expect(matchingBlocks('xyz', label)).toEqual([]);
  });

  it('moves around the list', () => {
    expect(moved(0, 1, 3)).toBe(1);
    expect(moved(2, 1, 3)).toBe(0);
    expect(moved(0, -1, 3)).toBe(2);
    expect(moved(0, 1, 0)).toBe(0);
  });
});
