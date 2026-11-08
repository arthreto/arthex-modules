<script lang="ts">
  import { Button, Icon, TextField, toast } from '@arthex/sdk';
  import { t } from './i18n';
  import { KEY_ICON } from './identity';
  import { openAddress, problemOf, removeKey, saveKey } from './library';

  interface Props {
    /** A key is saved: it can be removed, and the tool can go back to it. */
    saved: boolean;
    refused: boolean;
    onsaved: () => void;
    onremoved: () => void;
  }

  let { saved, refused: startRefused, onsaved, onremoved }: Props = $props();

  let draft = $state('');
  // svelte-ignore state_referenced_locally
  let refused = $state(startRefused);
  let saving = $state(false);

  async function save() {
    saving = true;
    try {
      await saveKey(draft.trim());
      onsaved();
    } catch (error) {
      if (problemOf(error) === 'refused') refused = true;
      else toast(t(problemOf(error) === 'offline' ? 'offline' : 'failed'));
    }
    saving = false;
  }

  async function remove() {
    try {
      await removeKey();
      onremoved();
    } catch {
      toast(t('failed'));
    }
  }

  function get() {
    openAddress('https://www.themoviedb.org/settings/api').catch(() => toast(t('failed')));
  }
</script>

<div class="page fade-in">
  <form
    class="box"
    onsubmit={(event) => {
      event.preventDefault();
      if (draft.trim() && !saving) void save();
    }}
  >
    <div class="heading">
      <span class="tile"><Icon path={KEY_ICON} size={20} /></span>
      <h3>{t('key.title')}</h3>
    </div>
    <div class:refused>
      <TextField
        label={t('key.field')}
        type="password"
        bind:value={draft}
        oninput={() => (refused = false)}
        placeholder={t('key.placeholder')}
        spellcheck="false"
        autocomplete="off"
        autofocus
      />
    </div>
    {#if refused}<span class="problem" role="status">{t('key.refused')}</span>{/if}
    <div class="buttons">
      <Button onclick={get}>{t('key.get')}</Button>
      <div class="end">
        {#if saved}<Button onclick={remove}>{t('key.remove')}</Button>{/if}
        <Button type="submit" variant="primary" disabled={!draft.trim() || saving}>{t('key.save')}</Button>
      </div>
    </div>
  </form>
</div>

<style>
  .page {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .box {
    width: 440px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .heading {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .tile {
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: 12px;
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    color: var(--tint-sky);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  h3 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }

  .refused :global(input) {
    border-color: var(--danger-text);
  }

  .problem {
    margin-top: -8px;
    font-size: 13px;
    color: var(--danger-text);
  }

  .buttons,
  .end {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
</style>
