<script lang="ts">
  import { Button, Icon, IconButton, Modal, Segmented, Switch } from '@arthex/sdk';
  import { forge } from './forge.svelte';
  import { t } from './i18n';

  let { onclose }: { onclose: () => void } = $props();

  const heights = $derived([
    { value: 'original', label: t('original') },
    { value: '1080', label: '1080p' },
    { value: '720', label: '720p' },
    { value: '480', label: '480p' },
  ]);
</script>

<Modal title={t('settings')} {onclose}>
  <label class="quality">
    <span class="between"
      ><span>{t('quality')}</span><span class="value">{forge.settings.quality} %</span></span
    >
    <input
      type="range"
      min="10"
      max="100"
      step="5"
      value={forge.settings.quality}
      onchange={(event) => forge.setSettings({ quality: event.currentTarget.valueAsNumber })}
    />
  </label>
  <div class="row">
    <span class="label">{t('resolution')}</span>
    <Segmented
      label={t('resolution')}
      options={heights}
      value={forge.settings.height === null ? 'original' : String(forge.settings.height)}
      onchange={(value) => forge.setSettings({ height: value === 'original' ? null : Number(value) })}
    />
  </div>
  <div class="row">
    <span class="label">{t('metadata')}</span>
    <Switch
      label={t('metadata')}
      checked={forge.settings.stripMetadata}
      onchange={(stripMetadata) => forge.setSettings({ stripMetadata })}
    />
  </div>
  <div class="row">
    <span class="folder">
      <span class="label">{t('outputFolder')}</span>
      <span class="path">{forge.settings.folder ?? t('nextToOriginal')}</span>
    </span>
    {#if forge.settings.folder}
      <IconButton
        icon="close"
        label={t('useOriginalFolder')}
        onclick={() => forge.setSettings({ folder: null })}
      />
    {/if}
    <Button onclick={() => forge.pickFolder()}>
      <Icon path="M3 7h6l2 2h10v10H3z" size={14} />{t('change')}
    </Button>
  </div>
</Modal>

<style>
  .quality {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .between {
    display: flex;
    justify-content: space-between;
    font-size: 14px;
    font-weight: 500;
  }

  .value {
    color: var(--text-muted);
  }

  input {
    width: 100%;
    accent-color: var(--accent);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .label {
    flex: 1;
    font-size: 14px;
    font-weight: 500;
  }

  .folder {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .path {
    overflow: hidden;
    font-size: 12.5px;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--text-subtle);
  }
</style>
