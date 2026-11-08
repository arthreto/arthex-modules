/** What a search result or the right-click menu passes to the tool, as one string. */
export type ToolStart =
  { tab: 'text'; text: string } | { tab: 'file'; compare: string } | { tab: 'password'; stored: string };

export const OPEN_FILE = 'file';

export const textInput = (text: string) => `text:${text}`;
export const compareInput = (hex: string) => `compare:${hex}`;
export const verifyInput = (stored: string) => `verify:${stored}`;

export function parseInput(input: string | undefined): ToolStart {
  if (input === OPEN_FILE) return { tab: 'file', compare: '' };
  if (input?.startsWith('compare:')) return { tab: 'file', compare: input.slice('compare:'.length) };
  if (input?.startsWith('verify:')) return { tab: 'password', stored: input.slice('verify:'.length) };
  return { tab: 'text', text: input?.startsWith('text:') ? input.slice('text:'.length) : '' };
}
