import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'path';

export default defineConfig({
  plugins: [react()],
  build: {
    emptyOutDir: false,
    lib: {
      entry: resolve(__dirname, 'src/cdn.ts'),
      name: 'RampOSWidget',
      fileName: (format) => `rampos-widget.${format}.js`,
      formats: ['umd', 'es'],
    },
    rollupOptions: {
      // Bundle everything for CDN
      external: [],
    },
  },
  define: {
    'process.env.NODE_ENV': '"production"',
  },
});
