import { defineModule } from '@arthex/sdk';
import { ICON, ID, TINT } from './identity';
import { unitsSearch } from './search';

export default defineModule({
  id: ID,
  version: '1.0.0',
  icon: ICON,
  tint: TINT,
  order: 61,
  tags: ['math', 'utilities'],
  locales: import.meta.glob('./locales/*.json', { eager: true, import: 'default' }),
  search: unitsSearch,
  tool: () => import('./UnitsTool.svelte'),
});
