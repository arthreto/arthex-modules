<script lang="ts">
  import { Button } from '@arthex/sdk';
  import GameHeader from './GameHeader.svelte';
  import Mascot from './Mascot.svelte';
  import { companion, type Item } from './companion.svelte';
  import { t } from './i18n';

  const SHOP: { id: Item; price: number }[] = [
    { id: 'scarf', price: 0 },
    { id: 'glasses', price: 150 },
    { id: 'bow', price: 200 },
    { id: 'crown', price: 500 },
  ];

  const mascot = $derived(companion.mascot);

  function act(item: Item): void {
    if (mascot?.wear === item) void companion.wear(null);
    else if (mascot?.owned.includes(item)) void companion.wear(item);
    else void companion.buy(item);
  }
</script>

{#if mascot?.species}
  <div class="shop">
    <GameHeader title={t('shop.title')} status={t('shop.balance', { count: mascot.kibble })} />
    <div class="items">
      {#each SHOP as item (item.id)}
        {@const owned = mascot.owned.includes(item.id)}
        {@const worn = mascot.wear === item.id}
        <div class="item">
          <Mascot species={mascot.species} wear={item.id} size={190} />
          <span class="name">{t(`shop.${item.id}`)}</span>
          <Button
            variant={owned ? 'default' : 'primary'}
            disabled={!owned && mascot.kibble < item.price}
            onclick={() => act(item.id)}
          >
            {#if worn}
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
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
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

  .name {
    font-size: 15px;
    font-weight: 600;
  }
</style>
