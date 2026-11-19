import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { createServer } from 'vite';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const output = resolve(root, 'src-tauri/icons/menu');
const icons = {
  capture: 'src/lib/components/CaptureIcon.svelte',
  'app-window-mac': 'node_modules/@lucide/svelte/dist/icons/app-window-mac.svelte',
  history: 'node_modules/@lucide/svelte/dist/icons/rotate-ccw-clock.svelte',
  clipboard: 'node_modules/@lucide/svelte/dist/icons/clipboard.svelte',
  list: 'node_modules/@lucide/svelte/dist/icons/list.svelte',
  power: 'node_modules/@lucide/svelte/dist/icons/power.svelte',
  info: 'node_modules/@lucide/svelte/dist/icons/info.svelte',
  processing: 'node_modules/@lucide/svelte/dist/icons/loader-circle.svelte',
  success: 'node_modules/@lucide/svelte/dist/icons/circle-check.svelte',
  cancelled: 'node_modules/@lucide/svelte/dist/icons/circle-x.svelte',
  error: 'node_modules/@lucide/svelte/dist/icons/triangle-alert.svelte',
};

const server = await createServer({
  root,
  configFile: false,
  cacheDir: resolve(root, '.svelte-kit/menu-icons-vite'),
  plugins: [svelte({ configFile: false })],
  server: { middlewareMode: true, hmr: false, ws: false, watch: null },
  appType: 'custom',
});

try {
  const { render } = await server.ssrLoadModule('svelte/server');
  await mkdir(output, { recursive: true });
  for (const [name, path] of Object.entries(icons)) {
    const { default: component } = await server.ssrLoadModule(resolve(root, path));
    const { body } = render(component, { props: { color: '#000', strokeWidth: 1.5 } });
    let svg = body
      .replace(/<!--[\s\S]*?-->/g, '')
      .replace(/currentColor/g, '#000')
      .replace(/<svg([^>]*)>/, (_, attributes) => {
        const normalized = attributes.replace(/\s(?:width|height|class)="[^"]*"/g, '');
        return `<svg width="24" height="24"${normalized}>`;
      })
      .trim();
    if (!svg.includes('xmlns=')) {
      svg = svg.replace('<svg ', '<svg xmlns="http://www.w3.org/2000/svg" ');
    }
    await writeFile(resolve(output, `${name}.svg`), `${svg}\n`);
  }
  await writeFile(
    resolve(output, 'LUCIDE-LICENSE'),
    await readFile(resolve(root, 'node_modules/@lucide/svelte/LICENSE')),
  );
} finally {
  await server.close();
}
