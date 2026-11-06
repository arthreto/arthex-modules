<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    /** Its place in the room: from the left wall, or from the right one on a wide screen. */
    left?: number;
    right?: number;
    dimmed?: boolean;
    onclick: () => void;
    children: Snippet;
  }

  let { label, left, right, dimmed = false, onclick, children }: Props = $props();
</script>

<button
  type="button"
  class="toy"
  class:dimmed
  aria-label={label}
  style:left={left === undefined ? undefined : `${left}px`}
  style:right={right === undefined ? undefined : `${right}px`}
  {onclick}
>
  <svg aria-hidden="true" width="92" height="92" viewBox="0 0 100 100">{@render children()}</svg>
  <span>{label}</span>
</button>

<style>
  .toy {
    position: absolute;
    bottom: 74px;
    width: 110px;
    margin-left: -55px;
    margin-right: -55px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    transition: transform 140ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .toy:hover {
    transform: translateY(-6px) scale(1.04);
  }

  .toy:active {
    transform: scale(0.96);
  }

  .dimmed {
    opacity: 0.5;
  }

  span {
    font-size: 13px;
    font-weight: 600;
  }
</style>
