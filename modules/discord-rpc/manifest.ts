import { defineModule } from '@arthex/sdk';
import { ICON, ID, TINT } from './identity';

export default defineModule({
  id: ID,
  version: '1.0.0',
  icon: ICON,
  tint: TINT,
  order: 64,
  tags: ['utilities', 'games'],
  locales: import.meta.glob('./locales/*.json', { eager: true, import: 'default' }),
  background: true,
  page: () => import('./RpcPage.svelte'),
});
