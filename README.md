# Arthex modules

The modules that [Arthex](https://github.com/arthreto/Arthex) users install from **Settings → Modules → Catalog**, or from a `.arthex` file. Every module here is built, signed with the Arthex update key and published by this repository's CI. Arthex refuses any package that is not signed with that key.

The modules that ship with Arthex itself (Home, FileSize, Notes, Clipboard, search…) live in the Arthex repository, not here.

## Layout

```
arthex/                  Arthex, as a submodule: its SDK (arthex/sdk) at a known version
modules/<id>/            one folder per module
  manifest.ts            what the module is and the elements it provides
  locales/<lang>.json    its texts: "name", "description", "keywords", then its own
  *.svelte               its views (page, Home card, tool…)
  *.test.ts              tests of its pure logic, run by `npm test`
  native/                its program, if it needs the system (optional Cargo project)
.github/workflows/       CI (every push) and Release (every change to modules/ on main)
```

## Set up

```bash
git clone --recurse-submodules https://github.com/arthreto/arthex-modules
```

```bash
npm install
```

A module with a program also needs [Rust](https://rustup.rs).

## Add a module

1. Create `modules/<id>/manifest.ts`. The `id` is lowercase `[a-z0-9-]`, at most 32 characters, and never changes once released.

   ```ts
   import { defineModule } from '@arthex/sdk';

   export default defineModule({
     id: 'timer',
     version: '1.0.0',
     icon: 'M12 7v5l3 2M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Z', // 24×24 stroked SVG path
     tint: 'green',
     order: 50,
     tags: ['time', 'productivity'], // its categories in the catalog, from the fixed list TAGS of the SDK
     locales: import.meta.glob('./locales/*.json', { eager: true, import: 'default' }),
     tool: () => import('./TimerTool.svelte'),
   });
   ```

2. Add `locales/en.json` and `locales/fr.json`, then an `i18n.ts`:

   ```ts
   import { moduleTranslator } from '@arthex/sdk';
   import type en from './locales/en.json';

   export const t = moduleTranslator<typeof en>('timer');
   ```

3. Write its views with what `@arthex/sdk` exports: the Arthex UI kit (`Button`, `Modal`, `Panel`, `Switch`…), `toast`, `openTool`, `navigate` and the formatting helpers.
4. If it needs the system, add a Cargo project in `native/` that depends on `arthex-module = { path = "../../../arthex/sdk/native" }`, and call it from the views with `moduleNative('<id>').call(method, params)`.

How each part works is described in `arthex/docs/DEVELOPMENT.md` → "Installable modules" and "Make an installable module".

## Try it in Arthex

Build it without signing, then start a development build of Arthex that loads it:

```bash
npm run build -- modules/<id> --out dev-modules
```

```bash
cd ../Arthex && ARTHEX_DEV_MODULES=<full path of arthex-modules/dev-modules> npm run tauri dev
```

## Release

Raise `version` in the module's `manifest.ts`, then push to `main`. The Release workflow packs every module, signs the packages and the catalog, and publishes them as the latest release, where Arthex reads them.

The repository needs two secrets (Settings → Secrets and variables → Actions), the same as the Arthex repository:

- `TAURI_SIGNING_PRIVATE_KEY`;
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

Programs are built for Windows x64.

## Update the SDK

```bash
git submodule update --remote arthex
```

Then commit the new `arthex` revision. A module packed with a newer SDK than a user's Arthex asks them to update Arthex first.

## License

PolyForm Noncommercial 1.0.0, as Arthex. See the [Arthex license](https://github.com/arthreto/Arthex/blob/main/LICENSE).
