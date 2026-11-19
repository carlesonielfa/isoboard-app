import js from '@eslint/js';
import globals from 'globals';
import svelte from 'eslint-plugin-svelte';
import svelteConfig from './svelte.config.js';
import ts from 'typescript-eslint';

export default ts.config(
  {
    ignores: [
      '.dev/**',
      'build/**',
      '.svelte-kit/**',
      'dist/**',
      'node_modules/**',
      'docs/**',
      'src-tauri/target/**',
      'src/lib/generated/**',
    ],
  },
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs['flat/recommended'],
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
  },
  {
    files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
    languageOptions: {
      parserOptions: {
        parser: ts.parser,
        svelteConfig,
      },
    },
  },
  ...svelte.configs['flat/prettier'],
  {
    files: ['src/**/*.{ts,js,svelte}'],
    ignores: ['src/lib/components/ui/**'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          paths: [
            {
              name: 'bits-ui',
              message:
                'Import a shared app component or ui primitive. Bits UI belongs only in src/lib/components/ui.',
            },
          ],
          patterns: [
            {
              group: ['bits-ui/*'],
              message:
                'Import a shared app component or ui primitive. Bits UI belongs only in src/lib/components/ui.',
            },
          ],
        },
      ],
    },
  },
);
