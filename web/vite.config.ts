import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// 开发时代理的 API 地址，可用 SHIYIN_API 覆盖（默认本机生产端口 6970）
const apiTarget = process.env.SHIYIN_API || 'http://127.0.0.1:6970';

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: Number(process.env.PORT) || 5173,
    strictPort: true,
    proxy: { '/api': apiTarget },
  },
  build: {
    target: 'es2022',
    rollupOptions: {
      input: {
        main: 'index.html',
        lyrics: 'lyrics.html',
        mini: 'mini.html',
      },
    },
    cssMinify: true,
    modulePreload: { polyfill: false },
    reportCompressedSize: true,
  },
});
