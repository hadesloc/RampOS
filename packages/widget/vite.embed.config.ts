import { defineConfig } from 'vite';
import { resolve } from 'path';

// Embed build config - Vanilla JS IIFE bundle (no React dependency)
export default defineConfig({
  build: {
    emptyOutDir: false,
    lib: {
      entry: resolve(__dirname, 'src/embed.ts'),
      name: 'RampOSWidget',
      fileName: (format) => `rampos-embed.${format}.js`,
      formats: ['iife', 'es'],
    },
    rollupOptions: {
      // No external dependencies - self-contained bundle
      external: [],
    },
  },
  define: {
    'process.env.NODE_ENV': '"production"',
  },
});
