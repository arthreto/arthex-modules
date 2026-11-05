<script lang="ts">
  import { Button, Icon, IconButton, Segmented } from '@arthex/sdk';
  import Field from './Field.svelte';
  import { t } from './i18n';
  import { presence } from './presence.svelte';
  import {
    KINDS,
    MAX_BUTTONS,
    MAX_LABEL_LENGTH,
    MAX_TEXT_LENGTH,
    MAX_URL_LENGTH,
    SHOWS,
    TIMES,
    addressError,
    appIdError,
    canTurnOn,
    fromLocalInput,
    parseCount,
    partyError,
    textError,
    toLocalInput,
    type Template,
    type TemplateButton,
  } from './template';

  let { template }: { template: Template } = $props();

  const active = $derived(presence.active === template.id);
  const kinds = $derived(KINDS.map((value) => ({ value, label: t(`kinds.${value}`) })));
  const shows = $derived(SHOWS.map((value) => ({ value, label: t(`shows.${value}`) })));
  const times = $derived(TIMES.map((value) => ({ value, label: t(`times.${value}`) })));

  const count = (value: number | null) => (value === null ? '' : String(value));

  function updateButton(index: number, changes: Partial<TemplateButton>) {
    presence.update({
      buttons: template.buttons.map((button, other) =>
        other === index ? { ...button, ...changes } : button,
      ),
    });
  }
</script>

<section class="form">
  <div class="head">
    <input
      class="name"
      aria-label={t('templateName')}
      value={template.name}
      oninput={(event) => presence.update({ name: event.currentTarget.value })}
    />
    <IconButton icon="copy" label={t('duplicate')} onclick={() => presence.duplicate(template)} />
    <IconButton icon="trash" label={t('remove')} onclick={() => presence.remove(template)} />
    <Button
      variant={active ? 'default' : 'primary'}
      disabled={!active && !canTurnOn(template)}
      onclick={() => presence.toggle(template)}
    >
      {active ? t('turnOff') : t('turnOn')}
    </Button>
  </div>

  <div class="sections scroll">
    <section class="group">
      <h4>{t('application')}</h4>
      <div class="grid two">
        <Field
          label={t('appId')}
          placeholder="1234567890"
          value={template.appId}
          error={appIdError(template.appId)}
          onvalue={(appId) => presence.update({ appId: appId.trim() })}
        />
        <Field
          label={t('shownName')}
          placeholder={t('appNameHint')}
          maxlength={MAX_TEXT_LENGTH}
          value={template.title}
          error={textError(template.title)}
          onvalue={(title) => presence.update({ title })}
        />
      </div>
      <div class="choices">
        <div class="choice">
          <span>{t('type')}</span>
          <Segmented
            label={t('type')}
            options={kinds}
            value={template.kind}
            onchange={(kind) => presence.update({ kind })}
          />
        </div>
        <div class="choice">
          <span>{t('status')}</span>
          <Segmented
            label={t('status')}
            options={shows}
            value={template.show}
            onchange={(show) => presence.update({ show })}
          />
        </div>
      </div>
    </section>

    <section class="group">
      <h4>{t('texts')}</h4>
      <div class="grid two">
        <Field
          label={t('details')}
          maxlength={MAX_TEXT_LENGTH}
          value={template.details}
          error={textError(template.details)}
          onvalue={(details) => presence.update({ details })}
        />
        <Field
          label={t('detailsLink')}
          placeholder="https://"
          maxlength={MAX_URL_LENGTH}
          value={template.detailsUrl}
          error={addressError(template.detailsUrl)}
          onvalue={(detailsUrl) => presence.update({ detailsUrl })}
        />
        <Field
          label={t('state')}
          maxlength={MAX_TEXT_LENGTH}
          value={template.state}
          error={textError(template.state)}
          onvalue={(state) => presence.update({ state })}
        />
        <Field
          label={t('stateLink')}
          placeholder="https://"
          maxlength={MAX_URL_LENGTH}
          value={template.stateUrl}
          error={addressError(template.stateUrl)}
          onvalue={(stateUrl) => presence.update({ stateUrl })}
        />
      </div>
    </section>

    <section class="group">
      <h4>{t('images')}</h4>
      <div class="grid three">
        <Field
          label={t('largeImage')}
          placeholder={t('imageHint')}
          value={template.large}
          onvalue={(large) => presence.update({ large })}
        />
        <Field
          label={t('hoverText')}
          maxlength={MAX_TEXT_LENGTH}
          value={template.largeText}
          error={textError(template.largeText)}
          onvalue={(largeText) => presence.update({ largeText })}
        />
        <Field
          label={t('link')}
          placeholder="https://"
          maxlength={MAX_URL_LENGTH}
          value={template.largeUrl}
          error={addressError(template.largeUrl)}
          onvalue={(largeUrl) => presence.update({ largeUrl })}
        />
        <Field
          label={t('smallImage')}
          placeholder={t('imageHint')}
          value={template.small}
          onvalue={(small) => presence.update({ small })}
        />
        <Field
          label={t('hoverText')}
          maxlength={MAX_TEXT_LENGTH}
          value={template.smallText}
          error={textError(template.smallText)}
          onvalue={(smallText) => presence.update({ smallText })}
        />
        <Field
          label={t('link')}
          placeholder="https://"
          maxlength={MAX_URL_LENGTH}
          value={template.smallUrl}
          error={addressError(template.smallUrl)}
          onvalue={(smallUrl) => presence.update({ smallUrl })}
        />
      </div>
    </section>

    <section class="group">
      <h4>{t('time')}</h4>
      <Segmented
        label={t('time')}
        options={times}
        value={template.time}
        onchange={(time) => presence.update({ time })}
      />
      {#if template.time === 'left'}
        <div class="grid two">
          <Field
            label={t('minutes')}
            type="number"
            placeholder="25"
            value={count(template.minutes)}
            onvalue={(minutes) => presence.update({ minutes: parseCount(minutes) })}
          />
        </div>
      {:else if template.time === 'dates'}
        <div class="grid two">
          <Field
            label={t('from')}
            type="datetime-local"
            value={toLocalInput(template.from)}
            onvalue={(from) => presence.update({ from: fromLocalInput(from) })}
          />
          <Field
            label={t('to')}
            type="datetime-local"
            value={toLocalInput(template.to)}
            onvalue={(to) => presence.update({ to: fromLocalInput(to) })}
          />
        </div>
      {/if}
    </section>

    <section class="group">
      <h4>{t('group')}</h4>
      <div class="grid two">
        <Field
          label={t('people')}
          type="number"
          placeholder="2"
          value={count(template.size)}
          error={partyError(template.size, template.max)}
          onvalue={(size) => presence.update({ size: parseCount(size) })}
        />
        <Field
          label={t('maximum')}
          type="number"
          placeholder="4"
          value={count(template.max)}
          onvalue={(max) => presence.update({ max: parseCount(max) })}
        />
      </div>
    </section>

    <section class="group last">
      <h4>{t('buttons')}</h4>
      {#each template.buttons as button, index (index)}
        <div class="button-row">
          <Field
            label={t('buttonLabel')}
            hideLabel
            placeholder={t('buttonLabel')}
            maxlength={MAX_LABEL_LENGTH}
            value={button.label}
            onvalue={(label) => updateButton(index, { label })}
          />
          <Field
            label={t('buttonAddress')}
            hideLabel
            placeholder="https://"
            maxlength={MAX_URL_LENGTH}
            value={button.url}
            error={addressError(button.url)}
            onvalue={(url) => updateButton(index, { url })}
          />
          <IconButton
            icon="close"
            label={t('removeButton')}
            onclick={() =>
              presence.update({ buttons: template.buttons.filter((_, other) => other !== index) })}
          />
        </div>
      {/each}
      {#if template.buttons.length < MAX_BUTTONS}
        <Button
          size="small"
          onclick={() => presence.update({ buttons: [...template.buttons, { label: '', url: '' }] })}
        >
          <Icon name="plus" size={14} stroke={2.2} />{t('addButton')}
        </Button>
      {/if}
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
    gap: 10px;
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

  .sections {
    flex: 1;
    min-height: 0;
    padding-right: 12px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .group.last {
    padding-bottom: 4px;
  }

  .group > :global(.button) {
    align-self: flex-start;
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

  .two {
    grid-template-columns: 1fr 1fr;
  }

  .three {
    grid-template-columns: 1.2fr 1fr 1fr;
  }

  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 22px;
  }

  .choice {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .button-row {
    display: grid;
    grid-template-columns: 2fr 3fr auto;
    align-items: start;
    gap: 10px;
  }
</style>
