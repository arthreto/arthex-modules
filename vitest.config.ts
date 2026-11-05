import { defineConfig } from 'vitest/config';

// Without this, Vitest would also run the tests of Arthex in its submodule.
export default defineConfig({
  test: { include: ['modules/**/*.test.ts'] },
});
