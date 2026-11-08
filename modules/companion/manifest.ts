import { defineModule } from '@arthex/sdk';
import { ICON, ID, TINT } from './identity';
import { openWhenAsked } from './opening';

openWhenAsked();

export default defineModule({
  id: ID,
  version: '1.1.0',
  api: 3,
  icon: ICON,
  tint: TINT,
  order: 40,
  tags: ['games'],
  locales: import.meta.glob('./locales/*.json', { eager: true, import: 'default' }),
  background: true,
  page: () => import('./CompanionPage.svelte'),
  widget: { slot: 'side', component: () => import('./CompanionCard.svelte') },
});
