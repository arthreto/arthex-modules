<script lang="ts">
  import { Segmented, type ToolProps } from '@arthex/sdk';
  import FileView from './FileView.svelte';
  import type { Format } from './formats';
  import { t } from './i18n';
  import { parseInput } from './input';
  import KeyView from './KeyView.svelte';
  import PasswordView from './PasswordView.svelte';
  import TextView from './TextView.svelte';

  type Tab = 'text' | 'file' | 'password' | 'key';

  let { input }: ToolProps = $props();

  // What a search result or the right-click passed on is only where the tool starts.
  // svelte-ignore state_referenced_locally
  const start = parseInput(input);

  let tab = $state<Tab>(start.tab);
  let format = $state<Format>('hex');

  const tabs = $derived<{ value: Tab; label: string }[]>([
    { value: 'text', label: t('tabs.text') },
    { value: 'file', label: t('tabs.file') },
    { value: 'password', label: t('tabs.password') },
    { value: 'key', label: t('tabs.key') },
  ]);
  const formats: { value: Format; label: string }[] = [
    { value: 'hex', label: 'hex' },
    { value: 'HEX', label: 'HEX' },
    { value: 'base64', label: 'Base64' },
  ];
</script>

<div class="tool">
  <div class="top">
    <Segmented options={tabs} value={tab} label={t('source')} onchange={(value) => (tab = value)} />
    {#if tab === 'text' || tab === 'file'}
      <Segmented
        options={formats}
        value={format}
        label={t('format')}
        onchange={(value) => (format = value)}
      />
    {/if}
  </div>
  {#if tab === 'text'}
    <TextView {format} text={start.tab === 'text' ? start.text : ''} />
  {:else if tab === 'file'}
    <FileView {format} compare={start.tab === 'file' ? start.compare : ''} />
  {:else if tab === 'password'}
    <PasswordView stored={start.tab === 'password' ? start.stored : ''} />
  {:else}
    <KeyView />
  {/if}
</div>

<style>
  .tool {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .top {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
</style>
