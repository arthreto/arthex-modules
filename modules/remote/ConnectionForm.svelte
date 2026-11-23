<script lang="ts">
  import { Button, IconButton, Segmented, Switch } from '@arthex/sdk';
  import { PROTOCOLS, authsOf, parsePort, withProtocol, type Connection, type Status } from './connection';
  import Field from './Field.svelte';
  import { t } from './i18n';
  import LetterField from './LetterField.svelte';
  import { placeOf, remote } from './remote.svelte';

  let { connection }: { connection: Connection } = $props();

  const EYE = 'M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12ZM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z';
  const EYE_OFF =
    'M3 3l18 18M10.6 5.1A10 10 0 0 1 12 5c6.5 0 10 7 10 7a17 17 0 0 1-2.8 3.7M6.6 6.6C3.8 8.4 2 12 2 12s3.5 7 10 7a9.5 9.5 0 0 0 5.4-1.6M9.9 9.9a3 3 0 0 0 4.2 4.2';

  const status = $derived(remote.statusOf(connection.id));
  const on = $derived(status.state === 'on');
  const busy = $derived(status.state === 'busy' || status.state === 'trust');
  const locked = $derived(on || busy);
  const held = $derived(remote.heldOf(connection.id));
  const place = $derived(placeOf(connection));
  const used = $derived(remote.connections.filter((other) => other.id !== connection.id).map((other) => other.letter));

  const protocols = $derived(PROTOCOLS.map((value) => ({ value, label: t(`protocols.${value}`), disabled: locked })));
  const auths = $derived(authsOf(connection.protocol).map((value) => ({ value, label: t(`auths.${value}`), disabled: locked })));

  // The form is made again for each connection, so these start from the one it shows.
  // svelte-ignore state_referenced_locally
  let portText = $state(String(connection.port));
  let password = $state('');
  let passphrase = $state('');
  let showPassword = $state(false);
  const portError = $derived(parsePort(portText) === null);

  function statusText(status: Status): string {
    switch (status.state) {
      case 'off':
        return t('status.off');
      case 'busy':
      case 'trust':
        return t('status.busy');
      case 'on':
        return place ? t('status.on', { place }) : t('status.onFolder');
      case 'error':
        return errorText(status.reason);
    }
  }

  function errorText(reason: string): string {
    switch (reason) {
      case 'refused':
        return t(connection.auth === 'key' ? 'errors.refusedKey' : 'errors.refusedPassword');
      case 'letterTaken':
        return t('errors.letterTaken', { letter: connection.letter });
      case 'unreachable':
      case 'changedHost':
      case 'certificate':
      case 'badKey':
      case 'noFolder':
      case 'system':
        return t(`errors.${reason}`);
      default:
        return t('errors.other');
    }
  }

  function onPort(text: string) {
    portText = text;
    const port = parsePort(text);
    if (port !== null) remote.update({ port });
  }
</script>

<section class="form">
  <div class="head">
    <input
      class="name"
      aria-label={t('connectionName')}
      value={connection.name}
      maxlength="40"
      disabled={locked}
      oninput={(event) => remote.update({ name: event.currentTarget.value })}
    />
    <IconButton icon="copy" label={t('duplicate')} onclick={() => remote.duplicate(connection)} />
    <IconButton icon="trash" label={t('remove')} onclick={() => (remote.removing = connection)} />
    {#if on}
      <Button onclick={() => remote.open(connection)}>{t('open', { place: place || connection.name })}</Button>
    {/if}
    <span class="main">
      <Button
        variant={locked ? 'default' : 'primary'}
        disabled={busy || (!on && (!remote.canConnect(connection) || portError))}
        onclick={() => remote.toggle(connection)}
      >
        {busy ? t('connecting') : on ? t('disconnect') : t('connect')}
      </Button>
    </span>
  </div>
  <span class="status" class:error={status.state === 'error'} role="status">
    <span class="dot {status.state}" aria-hidden="true"></span>{statusText(status)}
  </span>

  <div class="sections scroll">
    <section class="group">
      <h4>{t('sections.server')}</h4>
      <Segmented
        label={t('protocol')}
        options={protocols}
        value={connection.protocol}
        onchange={(protocol) => {
          const changes = withProtocol(connection, protocol);
          portText = String(changes.port);
          remote.update(changes);
        }}
      />
      <div class="grid host">
        <Field
          label={t('address')}
          placeholder={t('addressHint')}
          maxlength={255}
          value={connection.host}
          disabled={locked}
          onvalue={(host) => remote.update({ host: host.trim() })}
        />
        <Field
          label={t('port')}
          inputmode="numeric"
          maxlength={5}
          value={portText}
          error={portError ? '' : null}
          disabled={locked}
          onvalue={onPort}
        />
      </div>
    </section>

    <section class="group">
      <h4>{t('sections.login')}</h4>
      <Segmented
        label={t('auth')}
        options={auths}
        value={connection.auth}
        onchange={(auth) => remote.update({ auth })}
      />
      {#if connection.auth !== 'anonymous'}
        <div class="grid two">
          <Field
            label={t('user')}
            maxlength={255}
            value={connection.user}
            disabled={locked}
            onvalue={(user) => remote.update({ user })}
          />
          {#if connection.auth === 'password'}
            <div class="secret">
              <Field
                label={t('password')}
                type={showPassword ? 'text' : 'password'}
                placeholder={held.password ? t('savedSecret') : ''}
                value={password}
                disabled={locked}
                onvalue={(value) => {
                  password = value;
                  remote.setSecret('password', value);
                }}
              />
              <span class="eye">
                <IconButton
                  path={showPassword ? EYE_OFF : EYE}
                  label={showPassword ? t('hidePassword') : t('showPassword')}
                  active={showPassword}
                  onclick={() => (showPassword = !showPassword)}
                />
              </span>
            </div>
          {:else}
            <div class="key">
              <Field
                label={t('key')}
                mono
                readonly
                placeholder={t('noKey')}
                value={connection.keyName}
                error={remote.keyError ? t('errors.notKey') : null}
                disabled={locked}
              />
              <Button disabled={locked} onclick={() => remote.pickKey()}>{t('pickKey')}</Button>
            </div>
          {/if}
        </div>
        {#if connection.auth === 'key'}
          <div class="grid two">
            <span></span>
            <Field
              label={t('passphrase')}
              type="password"
              placeholder={held.passphrase ? t('savedSecret') : t('optional')}
              value={passphrase}
              disabled={locked}
              onvalue={(value) => {
                passphrase = value;
                remote.setSecret('passphrase', value);
              }}
            />
          </div>
        {/if}
      {/if}
    </section>

    <section class="group last">
      <h4>{t('sections.drive')}</h4>
      <div class="grid drive">
        <LetterField
          value={connection.letter}
          {used}
          disabled={locked}
          onchange={(letter) => remote.update({ letter })}
        />
        <Field
          label={t('folder')}
          mono
          maxlength={1024}
          value={connection.folder}
          disabled={locked}
          onvalue={(folder) => remote.update({ folder })}
        />
      </div>
      <div class="row">
        <span>{t('autoConnect')}</span>
        <Switch
          label={t('autoConnect')}
          checked={connection.autoConnect}
          onchange={(autoConnect) => remote.update({ autoConnect })}
        />
      </div>
    </section>
  </div>
</section>

<style>
  .form {
    flex: 1;
    min-width: 0;
    padding: 14px 8px 14px 20px;
    border-radius: 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .head {
    padding-right: 12px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .name {
    flex: 1;
    min-width: 0;
    padding: 2px 0;
    background: transparent;
    border: 0;
    outline: none;
    font: inherit;
    font-size: 20px;
    font-weight: 600;
    color: var(--text);
  }

  .main :global(.button) {
    min-width: 120px;
  }

  .status {
    min-height: 20px;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-muted);
  }

  .status.error {
    color: var(--tint-red);
  }

  .dot {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: 50%;
    background: var(--border-strong);
  }

  .dot.on {
    background: var(--tint-green);
  }

  .dot.busy,
  .dot.trust {
    background: var(--accent);
  }

  .dot.error {
    background: var(--tint-red);
  }

  .sections {
    flex: 1;
    min-height: 0;
    margin-top: 8px;
    padding-right: 12px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .group.last {
    padding-bottom: 4px;
  }

  h4 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }

  .grid {
    display: grid;
    gap: 10px 12px;
  }

  .host {
    grid-template-columns: 1fr 110px;
  }

  .two {
    grid-template-columns: 1fr 1fr;
  }

  .drive {
    grid-template-columns: 150px 1fr;
  }

  .secret {
    position: relative;
  }

  .secret :global(input) {
    padding-right: 40px;
  }

  .eye {
    position: absolute;
    right: 2px;
    bottom: 2px;
  }

  .key {
    display: flex;
    align-items: flex-end;
    gap: 8px;
  }

  .key > :global(.compact) {
    flex: 1;
  }

  .row {
    min-height: 40px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    font-size: 13.5px;
    color: var(--text);
  }
</style>
