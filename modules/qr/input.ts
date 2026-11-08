/** What a search result passes to the tool, as one string. */
export type ToolStart = { tab: 'create'; text: string } | { tab: 'read' };

export const READ = 'read';

export const createInput = (text: string) => `create:${text}`;

export function parseInput(input: string | undefined): ToolStart {
  if (input === READ) return { tab: 'read' };
  return { tab: 'create', text: input?.startsWith('create:') ? input.slice('create:'.length) : '' };
}
