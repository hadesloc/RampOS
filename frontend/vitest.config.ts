import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'],
    exclude: ['node_modules', '.next'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: [
        'node_modules/',
        'src/test/',
        '**/*.d.ts',
        '**/*.config.*',
        '.next/',
      ],
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      '@rampos/widget': path.resolve(__dirname, '../packages/widget/src/index'),
      'server-only': path.resolve(__dirname, './src/test/server-only.ts'),
      'next-intl/navigation': path.resolve(__dirname, './src/test/next-intl-navigation-mock.ts'),
      'next-intl': path.resolve(__dirname, './src/test/next-intl-mock.ts'),
    },
  },
})
