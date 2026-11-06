<script lang="ts">
  import { Button, Icon, IconButton, Segmented } from '@arthex/sdk';
  import { onMount } from 'svelte';
  import { calendar } from './calendar.svelte';
  import CalendarItemDialog from './CalendarItemDialog.svelte';
  import CalendarMonth from './CalendarMonth.svelte';
  import CalendarSmallMonths from './CalendarSmallMonths.svelte';
  import CalendarWeek from './CalendarWeek.svelte';
  import { LEVELS, type Level } from './dates';
  import { periodTitle } from './format';
  import { t } from './i18n';

  const PLUS = 'M12 5v14M5 12h14';
  /** One notch of a wheel can send several events: this keeps it to one step. */
  const ZOOM_REST_MS = 160;
  const CLOCK_MS = 60_000;

  const levels = $derived(LEVELS.map((value: Level) => ({ value, label: t(`levels.${value}`) })));

  /** By hand, not `onwheel`: the event must be cancellable for Ctrl + wheel to stay ours. */
  function zoomOnWheel(page: HTMLElement) {
    let lastZoom = 0;
    const onwheel = (event: WheelEvent) => {
      if (!event.ctrlKey) return;
      event.preventDefault();
      const now = Date.now();
      if (now - lastZoom < ZOOM_REST_MS) return;
      lastZoom = now;
      calendar.zoom(event.deltaY > 0 ? 1 : -1);
    };
    page.addEventListener('wheel', onwheel, { passive: false });
    return { destroy: () => page.removeEventListener('wheel', onwheel) };
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.defaultPrevented || (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight')) return;
    const target = event.target;
    if (target instanceof HTMLElement && target.closest('[role="dialog"]')) return;
    if (target instanceof HTMLInputElement && target.value !== '') return;
    event.preventDefault();
    calendar.move(event.key === 'ArrowRight' ? 1 : -1);
  }

  onMount(() => {
    void calendar.load();
    const clock = setInterval(() => calendar.tick(), CLOCK_MS);
    return () => clearInterval(clock);
  });
</script>

<svelte:window {onkeydown} />

<div class="page fade-in" use:zoomOnWheel>
  <header>
    <IconButton icon="chevronLeft" label={t('previous')} onclick={() => calendar.move(-1)} />
    <IconButton icon="chevronRight" label={t('next')} onclick={() => calendar.move(1)} />
    <h2>{periodTitle(calendar.level, calendar.focus)}</h2>
    <Button onclick={() => calendar.goToday()}>{t('today')}</Button>
    <Segmented
      options={levels}
      value={calendar.level}
      label={t('levels.label')}
      onchange={(level) => (calendar.level = level)}
    />
    <Button variant="primary" onclick={() => calendar.add()}>
      <Icon path={PLUS} size={14} />
      {t('add')}
    </Button>
  </header>
  {#if calendar.level === 'week'}
    <CalendarWeek />
  {:else if calendar.level === 'month'}
    <CalendarMonth />
  {:else}
    <CalendarSmallMonths year={calendar.level === 'year'} />
  {/if}
</div>

{#if calendar.draft}
  <CalendarItemDialog initial={calendar.draft} />
{/if}

<style>
  .page {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  header {
    flex: none;
    min-height: 36px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  h2 {
    flex: 1;
    min-width: 0;
    margin-left: 4px;
    font-size: 20px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
