# CLAUDE.md

Rules for anyone (human or AI) working on the Arthex modules. The rules of Arthex itself apply here too: read `arthex/CLAUDE.md` first, then `arthex/docs/DEVELOPMENT.md` → "Installable modules" and "Make an installable module".

- **No module is started without the owner's go-ahead.** The ideas for later are listed in `arthex/CLAUDE.md`.
- **Mockup first:** a module's screens are added to the Arthex mockup and validated by the owner before any code.
- One folder per module in `modules/<id>/`. The `id` never changes once released.
- Raise `version` in `manifest.ts` for every change users should receive.
- A module carries its own weight: its libraries stay in it, bundled in its package or in its `native/` program. Arthex is only changed for a generic brick that any module can use, never for a feature of one module.
- `api` in `manifest.ts` is the oldest version of the host API the module works with (1 by default): raise it only when it uses something newer, so that nobody has to update Arthex for nothing.
- Reuse the SDK: the UI kit, `toast`, `moduleTranslator` and `moduleNative`. A module never redefines a button, switch, modal or card. Its system access lives in its `native/` program.
- Every visible string goes through the module's locales, in English and French.
- Work and push directly on `main`. Commits are in English, short and imperative, authored as the owner (`arthreto <86776616+arthreto@users.noreply.github.com>`), with no co-author or session trailers.
- Two markdown files only: `README.md`, written for the users of the modules, and this file. In the README: never list every module: name a few followed by "etc.", or none, so that adding a module changes no README. How to build and release a module belongs in `arthex/docs/DEVELOPMENT.md`.
- The `arthex` submodule only moves with `git submodule update --remote arthex`, in a commit of its own.
