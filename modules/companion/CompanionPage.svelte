<script lang="ts">
  import { onMount } from 'svelte';
  import CompanionAdopt from './CompanionAdopt.svelte';
  import CompanionCatch from './CompanionCatch.svelte';
  import CompanionFlair from './CompanionFlair.svelte';
  import CompanionMemory from './CompanionMemory.svelte';
  import CompanionPet from './CompanionPet.svelte';
  import CompanionResult from './CompanionResult.svelte';
  import CompanionRoom from './CompanionRoom.svelte';
  import CompanionShop from './CompanionShop.svelte';
  import { companion } from './companion.svelte';

  const views = {
    home: CompanionRoom,
    shop: CompanionShop,
    result: CompanionResult,
    catch: CompanionCatch,
    memory: CompanionMemory,
    pet: CompanionPet,
    flair: CompanionFlair,
  };

  const View = $derived(views[companion.view]);

  onMount(() => {
    void companion.load();
  });
</script>

{#if companion.mascot}
  <div class="page">
    {#if companion.mascot.species}
      {#key companion.view}
        <View />
      {/key}
    {:else}
      <CompanionAdopt />
    {/if}
  </div>
{/if}

<style>
  .page {
    position: absolute;
    inset: 0;
    animation: enter 180ms ease-out;
  }

  @keyframes enter {
    from {
      opacity: 0;
      transform: scale(0.985);
    }
  }
</style>
