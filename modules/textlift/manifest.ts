import { defineModule } from '@arthex/sdk';
import { ICON, ID, TINT } from './identity';
import { openWhenAsked } from './opening';

const IMAGES = ['.png', '.jpg', '.jpeg', '.webp', '.bmp', '.gif', '.tif', '.tiff', '.heic'];

openWhenAsked();

export default defineModule({
  id: ID,
  version: '1.0.0',
  api: 6,
  icon: ICON,
  tint: TINT,
  order: 68,
  tags: ['utilities', 'productivity'],
  locales: import.meta.glob('./locales/*.json', { eager: true, import: 'default' }),
  background: true,
  tool: () => import('./TextLiftTool.svelte'),
  contextMenu: [{ id: 'get-text', targets: ['file'], extensions: IMAGES }],
});
