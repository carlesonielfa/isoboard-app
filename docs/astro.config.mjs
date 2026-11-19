// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightLlmsTxt from 'starlight-llms-txt';
import stripHtmlComments from './src/integrations/strip-html-comments.mjs';

const site = 'https://docs.isoboard.app';
const repository = 'https://github.com/carlesonielfa/isoboard-app';

export default defineConfig({
  site,
  trailingSlash: 'always',
  integrations: [
    stripHtmlComments(),
    starlight({
      title: 'isoboard',
      description:
        'How to install and use isoboard, the macOS app that turns screenshots and images into text, tables and formatting you can paste into other apps.',
      logo: { src: './src/assets/logo.svg', alt: 'isoboard' },
      favicon: '/favicon.svg',
      // English is the root locale; add `es: { label: 'Español', lang: 'es' }` when translations exist.
      defaultLocale: 'root',
      locales: { root: { label: 'English', lang: 'en' } },
      social: [
        { icon: 'github', label: 'GitHub', href: repository },
        { icon: 'external', label: 'isoboard.app', href: 'https://isoboard.app' },
      ],
      editLink: { baseUrl: `${repository}/edit/main/docs/` },
      lastUpdated: true,
      routeMiddleware: './src/routeData.ts',
      customCss: ['./src/styles/custom.css'],
      components: {
        ThemeProvider: './src/components/ThemeProvider.astro',
        ThemeSelect: './src/components/ThemeSelect.astro',
        SiteTitle: './src/components/SiteTitle.astro',
      },
      head: [
        { tag: 'meta', attrs: { property: 'og:site_name', content: 'isoboard Docs' } },
        {
          tag: 'link',
          attrs: { rel: 'alternate', type: 'text/plain', title: 'llms.txt', href: '/llms.txt' },
        },
      ],
      sidebar: [
        { label: 'Get started', items: [{ autogenerate: { directory: 'getting-started' } }] },
        { label: 'Use isoboard', items: [{ autogenerate: { directory: 'use' } }] },
        { label: 'Models and providers', items: [{ autogenerate: { directory: 'models' } }] },
        { label: 'License and purchase', items: [{ autogenerate: { directory: 'license' } }] },
        { label: 'Privacy and data', items: [{ autogenerate: { directory: 'privacy' } }] },
        { label: 'Troubleshooting', collapsed: true, items: [{ autogenerate: { directory: 'troubleshooting' } }] },
        { label: 'Reference', collapsed: true, items: [{ autogenerate: { directory: 'reference' } }] },
        { label: 'Community', items: [{ autogenerate: { directory: 'community' } }] },
        { label: 'Developers', collapsed: true, items: [{ autogenerate: { directory: 'developers' } }] },
      ],
      plugins: [
        starlightLlmsTxt({
          projectName: 'isoboard',
          description:
            'isoboard is a macOS menu bar app that converts screenshots into rich text, tables and images you can paste into other apps, using a local isOCR model or your own AI provider.',
          customSets: [
            {
              label: 'Developer documentation',
              description: 'Architecture, development setup and contribution docs for the isoboard source code.',
              paths: ['developers/**'],
            },
          ],
          exclude: ['developers/**'],
        }),
      ],
    }),
  ],
});
