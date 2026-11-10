import { defineModule } from '@arthex/sdk';
import { ICON, ID, TINT } from './identity';

export default defineModule({
  id: ID,
  version: '1.0.0',
  api: 9,
  uses: ['ffmpeg'],
  icon: ICON,
  tint: TINT,
  order: 72,
  tags: ['media', 'writing'],
  locales: import.meta.glob('./locales/*.json', {
    eager: true,
    import: 'default',
  }),
  tool: () => import('./TtsTool.svelte'),
});
