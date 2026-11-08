import { describe, expect, it } from 'vitest';
import { compareInput, OPEN_FILE, parseInput, textInput, verifyInput } from './input';

describe('parseInput', () => {
  it('opens the tab each input asks for', () => {
    expect(parseInput(undefined)).toEqual({ tab: 'text', text: '' });
    expect(parseInput(textInput('a:b c'))).toEqual({ tab: 'text', text: 'a:b c' });
    expect(parseInput(compareInput('abcd'))).toEqual({ tab: 'file', compare: 'abcd' });
    expect(parseInput(verifyInput('$2b$12$x'))).toEqual({ tab: 'password', stored: '$2b$12$x' });
    expect(parseInput(OPEN_FILE)).toEqual({ tab: 'file', compare: '' });
  });
});
