import { defineConfig, type Plugin } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'path';

const host = process.env.TAURI_DEV_HOST;

/**
 * Vite plugin that intercepts virtual CSS modules from node_modules
 * Svelte files before PostCSS tries to parse them. bits-ui has
 * components where the <script> content leaks into virtual CSS
 * module extraction, causing PostCSS parse errors.
 */
function skipNodeModulesCss(): Plugin {
  return {
    name: 'skip-node-modules-css',
    enforce: 'pre',
    transform(code, id) {
      if (
        id.includes('node_modules') &&
        id.includes('.svelte') &&
        id.includes('type=style') &&
        id.includes('lang.css')
      ) {
        // Return empty CSS instead of the broken extraction
        return { code: '', map: null };
      }
      return undefined;
    },
  };
}

export default defineConfig({
  plugins: [skipNodeModulesCss(), svelte()],
  clearScreen: false,
  resolve: {
    alias: {
      $lib: path.resolve('./src/lib'),
    },
    conditions: ['browser', 'svelte'],
  },
  optimizeDeps: {
    exclude: ['@lucide/svelte', 'bits-ui'],
  },
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: 'ws', host, port: 5174, overlay: false }
      : { overlay: false },
  },
});
