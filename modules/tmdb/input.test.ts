import { describe, expect, it } from 'vitest';
import { parseInput, searchInput } from './input';

describe('tool input', () => {
  it('opens a page or runs a search', () => {
    expect(parseInput('movie:693134')).toEqual({ page: 'movie:693134' });
    expect(parseInput(searchInput('dune'))).toEqual({ search: 'dune' });
    expect(parseInput(undefined)).toBeNull();
    expect(parseInput('nonsense')).toBeNull();
  });
});
