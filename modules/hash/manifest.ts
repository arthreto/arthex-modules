import { defineModule } from '@arthex/sdk';
import { ICON, ID, TINT } from './identity';
import { openWhenAsked } from './opening';
import { hashSearch } from './search';

openWhenAsked();

export default defineModule({
  id: ID,
  version: '1.0.1',
  api: 6,
  icon: ICON,
  tint: TINT,
  order: 69,
  tags: ['developer', 'utilities'],
  locales: import.meta.glob('./locales/*.json', { eager: true, import: 'default' }),
  // A large file goes on being hashed with no call from the interface for minutes.
  background: true,
  search: hashSearch,
  tool: () => import('./HashTool.svelte'),
  contextMenu: [{ id: 'hash', targets: ['file'] }],
});
