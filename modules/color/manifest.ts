import { defineModule } from '@arthex/sdk';
import { ICON, ID, TINT } from './identity';
import { colorSearch } from './search';

export default defineModule({
  id: ID,
  version: '1.0.0',
  icon: ICON,
  tint: TINT,
  order: 62,
  tags: ['design', 'utilities', 'developer'],
  locales: import.meta.glob('./locales/*.json', { eager: true, import: 'default' }),
  search: colorSearch,
  tool: () => import('./ColorTool.svelte'),
});
