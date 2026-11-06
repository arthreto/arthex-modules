import { defineModule } from '@arthex/sdk';
import { t } from './i18n';
import { ICON, ID, TINT } from './identity';

export default defineModule({
  id: ID,
  version: '1.0.0',
  api: 6,
  icon: ICON,
  tint: TINT,
  order: 30,
  tags: ['productivity', 'time'],
  locales: import.meta.glob('./locales/*.json', { eager: true, import: 'default' }),
  background: true,
  page: () => import('./CalendarPage.svelte'),
  widget: { slot: 'side', component: () => import('./CalendarCard.svelte') },
  hints: () => [
    { keys: ['←', '→'], label: t('hints.period') },
    { keys: [t('hints.ctrl'), t('hints.wheel')], together: true, label: t('hints.zoom') },
  ],
});
