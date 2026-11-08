import { defineModule } from '@arthex/sdk';
import { ICON, ID, TINT } from './identity';
import { tmdbSearch } from './search';

export default defineModule({
  id: ID,
  version: '1.0.0',
  api: 6,
  icon: ICON,
  tint: TINT,
  order: 71,
  tags: ['media'],
  locales: import.meta.glob('./locales/*.json', { eager: true, import: 'default' }),
  search: tmdbSearch,
  tool: () => import('./TmdbTool.svelte'),
});
