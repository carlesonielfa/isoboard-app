import type { KnipConfig } from 'knip';
import { compile } from 'svelte/compiler';

export default {
  project: ['src/**/*.{js,ts,svelte,css}', '!src/lib/generated/**', 'tests/**/*.ts', '*.{js,ts}'],
  compilers: {
    svelte: (source, filename) => compile(source, { filename, generate: 'client' }).js.code,
  },
} satisfies KnipConfig;
