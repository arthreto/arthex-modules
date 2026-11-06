import { moduleNative, navigate } from '@arthex/sdk';
import { ID } from './identity';

/** Arthex registers a module once all of them are loaded, and only then can it open its page. */
const REGISTERED_AFTER_MS = 1000;

/**
 * Opens the page after a click on "Play" on the screen. It runs while the module loads, so that it
 * works with the page closed. `arthex build` loads the module outside Arthex too, where there is
 * nothing to do.
 */
export function openWhenAsked(): void {
  if (!('__ARTHEX__' in globalThis)) return;
  const native = moduleNative(ID);
  native.on('open', () => navigate(ID));
  native
    .call<boolean>('takeOpen')
    .then((asked) => {
      if (asked) setTimeout(() => navigate(ID), REGISTERED_AFTER_MS);
    })
    .catch(() => undefined);
}
