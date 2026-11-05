<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { presence } from './presence.svelte';
  import PresencePreview from './PresencePreview.svelte';
  import StartChoice from './StartChoice.svelte';
  import TemplateForm from './TemplateForm.svelte';
  import TemplateList from './TemplateList.svelte';

  onMount(() => void presence.load());
  onDestroy(() => presence.saveNow());
</script>

<div class="page fade-in">
  <TemplateList />
  {#if presence.current}
    <TemplateForm template={presence.current} />
  {:else}
    <StartChoice />
  {/if}
  <PresencePreview template={presence.current} />
</div>

<style>
  .page {
    position: absolute;
    inset: 0;
    display: flex;
    gap: 16px;
  }
</style>
