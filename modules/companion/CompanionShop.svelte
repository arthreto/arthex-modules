<script lang="ts">
  import { Button, Icon, Segmented } from '@arthex/sdk';
  import GameArt from './GameArt.svelte';
  import GameHeader from './GameHeader.svelte';
  import Mascot from './Mascot.svelte';
  import { companion, type Item, type ShopGame } from './companion.svelte';
  import { t } from './i18n';
  import { LOCK_ICON } from './identity';
  import shop from './shop.json';

  type Tab = 'games' | 'outfits';

  const GAMES = shop.games as { id: ShopGame; price: number; xp: number; kibble: number }[];
  const OUTFITS = shop.wear as { id: Item; price: number }[];

  const mascot = $derived(companion.mascot);
  let tab = $state<Tab>('games');
  const tabs = $derived<{ value: Tab; label: string }[]>([
    { value: 'games', label: t('shop.games') },
    { value: 'outfits', label: t('shop.outfits') },
  ]);

  function act(item: Item): void {
    if (mascot?.wear === item) void companion.wear(null);
    else if (mascot?.owned.includes(item)) void companion.wear(item);
    else void companion.buy(item);
  }
</script>

{#if mascot?.species}
  <div class="shop">
    <GameHeader title={t('shop.title')} status={t('shop.balance', { count: mascot.kibble })}>
      <Segmented options={tabs} value={tab} label={t('shop.title')} onchange={(value) => (tab = value)} />
    </GameHeader>
    {#if tab === 'games'}
      <div class="items games">
        {#each GAMES as game (game.id)}
          {@const owned = companion.owns(game.id)}
          <div class="item">
            <span class:locked={!owned}><GameArt game={game.id} size={120} /></span>
            <span class="name">
              {#if !owned}
                <Icon path={LOCK_ICON} size={15} />
              {/if}
              {t(`games.${game.id}`)}
            </span>
            <span class="gains">
              <span class="xp">+{game.xp} XP</span>
              <span class="kibble">{t('kibble', { count: game.kibble })}</span>
            </span>
            <Button
              variant={owned ? 'default' : 'primary'}
              disabled={!owned && mascot.kibble < game.price}
              onclick={() => (owned ? companion.play(game.id) : void companion.buyGame(game.id))}
            >
              {owned ? t('play') : t('shop.price', { count: game.price })}
            </Button>
          </div>
        {/each}
      </div>
    {:else}
      <div class="items outfits">
        {#each OUTFITS as item (item.id)}
          {@const owned = mascot.owned.includes(item.id)}
          <div class="item">
            <Mascot species={mascot.species} wear={item.id} size={130} />
            <span class="label">{t(`shop.${item.id}`)}</span>
            <Button
              size="small"
              variant={owned ? 'default' : 'primary'}
              disabled={!owned && mascot.kibble < item.price}
              onclick={() => act(item.id)}
            >
              {#if mascot.wear === item.id}
                {t('shop.remove')}
              {:else if owned}
                {t('shop.wear')}
              {:else}
                {t('shop.price', { count: item.price })}
              {/if}
            </Button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .shop {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .items {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 10px;
  }

  .outfits {
    grid-auto-rows: 226px;
    padding-right: 4px;
    overflow: auto;
    scrollbar-width: thin;
  }

  .item {
    padding: 12px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .outfits .item {
    padding: 10px;
    gap: 6px;
  }

  .locked {
    opacity: 0.5;
  }

  .name {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 15px;
    font-weight: 600;
  }

  .label {
    font-size: 14px;
    font-weight: 600;
    text-align: center;
  }

  .gains {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    font-size: 13px;
    font-weight: 600;
  }

  .xp {
    color: var(--accent);
  }

  .kibble {
    color: var(--tint-orange);
  }
</style>
