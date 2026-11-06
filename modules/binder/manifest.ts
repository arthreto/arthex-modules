import { defineModule } from '@arthex/sdk';
import { ICON, ID, TINT } from './identity';

export default defineModule({
  id: ID,
  version: '1.0.0',
  api: 8,
  icon: ICON,
  tint: TINT,
  order: 35,
  tags: ['writing', 'education', 'productivity'],
  locales: import.meta.glob('./locales/*.json', { eager: true, import: 'default' }),
  page: () => import('./BinderPage.svelte'),
});
