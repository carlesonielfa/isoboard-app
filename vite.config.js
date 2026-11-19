import { defineConfig } from 'vite';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import process from 'node:process';
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [
    tailwindcss(),
    // Tauri has no Node.js server for SSR, so adapter-static with an index.html
    // fallback runs the app in SPA mode.
    // See: https://svelte.dev/docs/kit/single-page-apps
    // See: https://v2.tauri.app/start/frontend/sveltekit/
    sveltekit({
      preprocess: vitePreprocess(),
      // SvelteKit 3 replaces `$lib` with `#lib`; keep the existing imports and
      // shadcn-svelte aliases working.
      alias: { $lib: 'src/lib' },
      adapter: adapter({
        fallback: 'index.html',
      }),
    }),
  ],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || '127.0.0.1',
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri` and the generated dev app bundle
      ignored: ['**/src-tauri/**', '**/.dev/**'],
    },
  },
}));
