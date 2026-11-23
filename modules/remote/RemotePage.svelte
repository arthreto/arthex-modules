<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import ConnectionForm from './ConnectionForm.svelte';
  import ConnectionList from './ConnectionList.svelte';
  import Dialogs from './Dialogs.svelte';
  import { remote } from './remote.svelte';
  import StartChoice from './StartChoice.svelte';

  onMount(() => void remote.load());
  onDestroy(() => void remote.saveNow());
</script>

<div class="page fade-in">
  <ConnectionList />
  {#if remote.current}
    {#key remote.current.id}
      <ConnectionForm connection={remote.current} />
    {/key}
  {:else}
    <StartChoice />
  {/if}
</div>
<Dialogs />

<style>
  .page {
    position: absolute;
    inset: 0;
    display: flex;
    gap: 16px;
  }
</style>
