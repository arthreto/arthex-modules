import { parseKey, type MediaKey } from './media';

/** What a search result passes to the tool: a page to open, or a search to run. */
export type ToolStart = { page: MediaKey } | { search: string } | null;

export const searchInput = (query: string) => `search:${query}`;

export function parseInput(input: string | undefined): ToolStart {
  if (!input) return null;
  if (input.startsWith('search:')) return { search: input.slice('search:'.length) };
  return parseKey(input) ? { page: input as MediaKey } : null;
}
