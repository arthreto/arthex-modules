import { defineModule } from '@arthex/sdk';
import { ICON, ID, TINT } from './identity';
import { qrSearch } from './search';

export default defineModule({
  id: ID,
  version: '1.0.0',
  // The QR code of a search result is shown in its preview.
  api: 10,
  icon: ICON,
  tint: TINT,
  order: 70,
  tags: ['utilities', 'productivity'],
  locales: import.meta.glob('./locales/*.json', { eager: true, import: 'default' }),
  search: qrSearch,
  tool: () => import('./QrTool.svelte'),
});
