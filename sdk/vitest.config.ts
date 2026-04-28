import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'jsdom',
    globals: true,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html', 'lcov'],
      thresholds: {
        lines: 60,
        functions: 60,
        branches: 50,
        statements: 60,
      },
      include: ['src/**/*.ts'],
      exclude: ['src/**/*.d.ts', 'src/plugins/behavior.ts', 'src/plugins/profile.ts'],
    },
  },
});
