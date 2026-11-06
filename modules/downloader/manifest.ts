import { defineModule } from '@arthex/sdk';
import { ICON, ID, TINT } from './identity';
import { downloaderSearch } from './search';

export default defineModule({
  id: ID,
  version: '1.0.0',
  api: 9,
  uses: ['ffmpeg'],
  icon: ICON,
  tint: TINT,
  order: 67,
  tags: ['media', 'utilities', 'network'],
  locales: import.meta.glob('./locales/*.json', { eager: true, import: 'default' }),
  // A download goes on with the overlay closed.
  background: true,
  search: downloaderSearch,
  tool: () => import('./DownloaderTool.svelte'),
});
