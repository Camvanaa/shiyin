import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    proxy: { '/api': 'http://127.0.0.1:6970' },
  },
  build: {
    target: 'es2022',
    cssMinify: true,
    modulePreload: { polyfill: false },
    reportCompressedSize: true,
  },
});
