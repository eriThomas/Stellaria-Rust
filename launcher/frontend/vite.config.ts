import { resolve } from 'node:path';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [react()],
  base: './',
  build: {
    outDir: resolve(__dirname, '../dist'),
    emptyOutDir: false,
    rollupOptions: {
      input: {
        dashboard: resolve(__dirname, 'dashboard.html'),
        bar: resolve(__dirname, 'bar.html'),
      },
      output: {
        entryFileNames: 'dashboard/[name].js',
        chunkFileNames: 'dashboard/chunks/[name]-[hash].js',
        assetFileNames: 'dashboard/assets/[name]-[hash][extname]',
      },
    },
  },
});
