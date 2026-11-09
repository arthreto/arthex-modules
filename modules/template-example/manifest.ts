import { defineModule } from '@arthex/sdk';
import { watchCreations } from './creations';
import { ICON, ID, TINT } from './identity';

watchCreations();

export default defineModule({
  id: ID,
  version: '1.0.0',
  api: 9,
  uses: ['ffmpeg'],
  icon: ICON,
  tint: TINT,
  order: 69,
  tags: ['files', 'developer'],
  locales: import.meta.glob('./locales/*.json', { eager: true, import: 'default' }),
  tool: () => import('./TemplateTool.svelte'),
  contextMenu: [{ id: 'create', targets: ['folder', 'folder-background'] }],
});
