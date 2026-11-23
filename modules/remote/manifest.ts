import { defineModule } from '@arthex/sdk';
import { ICON, ID, TINT } from './identity';
import { remoteSearch } from './search';

export default defineModule({
  id: ID,
  version: '1.0.0',
  // The key is picked in a window of the program, with the overlay out of the way.
  api: 3,
  icon: ICON,
  tint: TINT,
  order: 72,
  tags: ['network', 'files', 'developer'],
  locales: import.meta.glob('./locales/*.json', { eager: true, import: 'default' }),
  // A drive stays connected with the overlay closed.
  background: true,
  search: remoteSearch,
  page: () => import('./RemotePage.svelte'),
});
