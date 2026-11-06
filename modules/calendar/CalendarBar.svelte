<script lang="ts">
  import { calendar } from './calendar.svelte';
  import { itemDescription } from './format';
  import type { Placed } from './layout';

  interface Props {
    part: Placed;
    size: 'large' | 'small' | 'line';
    top: number;
  }

  let { part, size, top }: Props = $props();

  const isEvent = $derived(part.item.kind === 'event');
  const insetLeft = $derived(part.opens ? 3 : 0);
  const insetRight = $derived(part.closes ? 3 : 0);
  const span = $derived(part.last - part.first + 1);
  const description = $derived(itemDescription(part.item));

  function open(event: MouseEvent) {
    event.stopPropagation();
    calendar.edit(part.item);
  }
</script>

<button
  type="button"
  class="bar {size}"
  class:event={isEvent}
  class:opens={part.opens}
  class:closes={part.closes}
  class:over={part.over}
  style:--tint="var(--tint-{part.item.tint})"
  style:left="calc({(part.first / 7) * 100}% + {insetLeft}px)"
  style:width="calc({(span / 7) * 100}% - {insetLeft + insetRight}px)"
  style:top="{top}px"
  title={description}
  aria-label={description}
  ondblclick={(event) => event.stopPropagation()}
  onclick={open}
>
  {#if isEvent}
    <span class="dot"></span>
  {:else}
    <span class="gone" style:width="{part.elapsed * 100}%"></span>
  {/if}
  {#if size !== 'line'}
    <span class="label">{part.item.title}</span>
  {/if}
</button>

<style>
  .bar {
    position: absolute;
    overflow: hidden;
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 6px;
    background: color-mix(in srgb, var(--tint) 24%, transparent);
    color: var(--text);
    white-space: nowrap;
    text-align: left;
  }

  .opens {
    border-top-left-radius: 5px;
    border-bottom-left-radius: 5px;
  }

  .closes {
    border-top-right-radius: 5px;
    border-bottom-right-radius: 5px;
  }

  .bar:hover {
    background: color-mix(in srgb, var(--tint) 34%, transparent);
  }

  .large {
    height: 18px;
    font-size: 11.5px;
  }

  .small {
    height: 15px;
    font-size: 10.5px;
  }

  .line {
    height: 2px;
    padding: 0;
  }

  .event {
    background: transparent;
  }

  .event:hover {
    background: var(--surface-hover);
  }

  .over {
    background: color-mix(in srgb, var(--tint) 14%, transparent);
    color: var(--text-subtle);
  }

  .gone {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    background: color-mix(in srgb, var(--tint) 34%, transparent);
  }

  .over .gone {
    background: color-mix(in srgb, var(--tint) 10%, transparent);
  }

  .dot {
    position: relative;
    width: 6px;
    height: 6px;
    flex: none;
    border-radius: 50%;
    background: var(--tint);
  }

  .label {
    position: relative;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
