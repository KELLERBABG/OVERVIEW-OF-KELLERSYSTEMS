import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    testTimeout: 15000,
    hookTimeout: 15000,
    globals: false,
    include: ['tests/**/*.test.ts'],
  },
});
