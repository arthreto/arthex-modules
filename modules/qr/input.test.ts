import { describe, expect, it } from 'vitest';
import { createInput, parseInput, READ } from './input';

describe('tool input', () => {
  it('opens the tab a search result asked for', () => {
    expect(parseInput(undefined)).toEqual({ tab: 'create', text: '' });
    expect(parseInput(createInput('a:b'))).toEqual({ tab: 'create', text: 'a:b' });
    expect(parseInput(READ)).toEqual({ tab: 'read' });
  });
});
