<script lang="ts">
  import { Icon } from '@arthex/sdk';
  import GameArt from './GameArt.svelte';
  import { companion, type ShopGame } from './companion.svelte';
  import { t } from './i18n';
  import { LOCK_ICON } from './identity';

  interface Props {
    game: ShopGame;
    /** Its place in the room: from the left wall, or from the right one on a wide screen. */
    left?: number;
    right?: number;
    dimmed?: boolean;
  }

  let { game, left, right, dimmed = false }: Props = $props();

  const locked = $derived(!companion.owns(game));
  const name = $derived(t(`games.${game}`));
</script>

<button
  type="button"
  class="toy"
  aria-label={locked ? t('locked', { name }) : name}
  style:left={left === undefined ? undefined : `${left}px`}
  style:right={right === undefined ? undefined : `${right}px`}
  onclick={() => (locked ? (companion.view = 'shop') : companion.play(game))}
>
  <span class="art" class:dimmed class:locked><GameArt {game} size={92} /></span>
  <span class="name">
    {#if locked}
      <Icon path={LOCK_ICON} size={14} />
    {/if}
    {name}
  </span>
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

  .locked {
    opacity: 0.45;
  }

  .name {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 13px;
    font-weight: 600;
  }
</style>
