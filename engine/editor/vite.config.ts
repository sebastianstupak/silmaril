import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'path';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  resolve: {
    alias: {
      $lib: path.resolve('./src/lib'),
    },
    conditions: ['svelte', 'default'],
  },
  optimizeDeps: {
    exclude: ['@lucide/svelte', 'bits-ui'],
  },
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: 'ws', host, port: 5174 }
      : undefined,
  },
});
