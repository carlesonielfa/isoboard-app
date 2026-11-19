import { defineRouteMiddleware } from '@astrojs/starlight/route-data';
import { existsSync } from 'node:fs';
import { join } from 'node:path';

const SITE = 'https://docs.isoboard.app';
const ORG_ID = 'https://isoboard.app/#organization';
const APP_ID = 'https://isoboard.app/#software';
const OG_IMAGE_PATH = '/media/hero-poster.png';
const hasOgImage = existsSync(join(process.cwd(), 'public', OG_IMAGE_PATH));

const organization = {
  '@type': 'Organization',
  '@id': ORG_ID,
  name: 'isoboard',
  url: 'https://isoboard.app/',
};

// Pricing mirrors license/pricing-and-licensing: personal use is free and work
// use is a one-time per-user license. Tax display varies by region, so the
// commercial offer states the base price and defers tax to checkout.
const softwareApplication = {
  '@type': 'SoftwareApplication',
  '@id': APP_ID,
  name: 'isoboard',
  description:
    'A macOS menu bar app that converts screenshots into rich text, tables and images you can paste into other apps, using a local isOCR model or your own AI provider.',
  url: 'https://isoboard.app/',
  operatingSystem: 'macOS 14 or later',
  applicationCategory: 'UtilitiesApplication',
  publisher: { '@id': ORG_ID },
  offers: [
    {
      '@type': 'Offer',
      name: 'Personal use',
      price: '0',
      priceCurrency: 'EUR',
      url: `${SITE}/license/pricing-and-licensing/`,
    },
    {
      '@type': 'Offer',
      name: 'Commercial license, one payment per user',
      price: '50',
      priceCurrency: 'EUR',
      description: 'One-time payment per user for work use. Tax is shown at checkout depending on region.',
      url: `${SITE}/license/pricing-and-licensing/`,
    },
  ],
};

const SOFTWARE_PAGES = new Set(['getting-started/what-is-isoboard']);

/** Plain text from a Markdown fragment. */
function plain(md: string): string {
  return md
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/^:::.*$/gm, '')
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, '')
    .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/[*_]{1,3}([^*_]+)[*_]{1,3}/g, '$1')
    .replace(/`([^`]*)`/g, '$1')
    .replace(/^\s*[-*]\s+/gm, '')
    .replace(/\s+/g, ' ')
    .trim();
}

/** Question and answer pairs from bold-line questions (a paragraph that is only `**Question?**`). */
function extractFaq(body: string | undefined) {
  if (!body) return [];
  const pairs: { q: string; a: string[] }[] = [];
  let current: { q: string; a: string[] } | undefined;
  let fenced = false;
  for (const block of body.split(/\n{2,}/)) {
    if ((block.match(/^```/gm) ?? []).length % 2 === 1) fenced = !fenced;
    if (fenced) continue;
    const q = block.trim().match(/^\*\*([^*\n]+\?)\*\*$/);
    if (q) {
      current = { q: q[1], a: [] };
      pairs.push(current);
    } else if (/^#{1,6}\s/.test(block.trim())) {
      current = undefined;
    } else if (current && !/^\|/.test(block.trim())) {
      current.a.push(block);
    }
  }
  return pairs
    .map(({ q, a }) => ({ q, a: plain(a.join('\n\n')) }))
    .filter(({ a }) => a.length > 0);
}

/** First page link under a sidebar group, used as the section landing page. */
function firstLink(entries: any[]): { href: string } | undefined {
  for (const e of entries) {
    if (e.type === 'link') return e;
    const found = firstLink(e.entries);
    if (found) return found;
  }
}

/** Sidebar group labels leading to the current page. */
function findTrail(entries: any[], trail: any[] = []): any[] | undefined {
  for (const e of entries) {
    if (e.type === 'link' && e.isCurrent) return trail;
    if (e.type === 'group') {
      const found = findTrail(e.entries, [...trail, e]);
      if (found) return found;
    }
  }
}

export const onRequest = defineRouteMiddleware((context) => {
  const route = context.locals.starlightRoute;
  const { id, entry, head } = route;
  // Starlight defaults to a large-image card. Without an og:image, a plain summary card is accurate.
  if (!hasOgImage) {
    const card = head.find((h) => h.tag === 'meta' && h.attrs?.name === 'twitter:card');
    if (card?.attrs) card.attrs.content = 'summary';
  }
  if (id === '404') {
    head.push({ tag: 'meta', attrs: { name: 'robots', content: 'noindex' } });
    return;
  }
  if (!entry?.data) return;

  const { title, description } = entry.data;
  const url = new URL(context.url.pathname, SITE).href;
  const isHome = context.url.pathname === '/';

  if (hasOgImage) {
    const image = new URL(OG_IMAGE_PATH, SITE).href;
    head.push(
      { tag: 'meta', attrs: { property: 'og:image', content: image } },
      { tag: 'meta', attrs: { name: 'twitter:image', content: image } },
    );
  }

  const graph: Record<string, unknown>[] = [];

  if (isHome) {
    graph.push(organization, {
      '@type': 'WebSite',
      '@id': `${SITE}/#website`,
      name: 'isoboard Docs',
      url: `${SITE}/`,
      inLanguage: 'en',
      publisher: { '@id': ORG_ID },
    });
  }
  if (isHome || SOFTWARE_PAGES.has(id)) graph.push(softwareApplication);

  if (!isHome) {
    const trail = findTrail(route.sidebar) ?? [];
    const items: { name: string; item: string }[] = [{ name: 'isoboard Docs', item: `${SITE}/` }];
    for (const group of trail) {
      const link = firstLink(group.entries);
      if (link) items.push({ name: group.label, item: new URL(link.href, SITE).href });
    }
    items.push({ name: title, item: url });
    // When the section landing page is the current page, keep the page entry.
    const unique = items.filter((it, i) => items.findLastIndex((o) => o.item === it.item) === i);
    graph.push({
      '@type': 'BreadcrumbList',
      itemListElement: unique.map((it, i) => ({
        '@type': 'ListItem',
        position: i + 1,
        name: it.name,
        item: it.item,
      })),
    });
  }

  if (id.startsWith('developers/')) {
    graph.push({
      '@type': 'TechArticle',
      headline: title,
      description,
      url,
      inLanguage: 'en',
      mainEntityOfPage: url,
      ...(route.lastUpdated ? { dateModified: route.lastUpdated.toISOString() } : {}),
      author: { '@id': ORG_ID },
      publisher: { '@id': ORG_ID },
    });
  }

  const faq = extractFaq(entry.body);
  if (faq.length >= 2) {
    graph.push({
      '@type': 'FAQPage',
      url,
      mainEntity: faq.map(({ q, a }) => ({
        '@type': 'Question',
        name: q,
        acceptedAnswer: { '@type': 'Answer', text: a },
      })),
    });
  }

  if (graph.length === 0) return;
  // Organization is defined once on the home page. Pages that reference it by id
  // carry a stub so each page's graph resolves on its own.
  if (!isHome && JSON.stringify(graph).includes(ORG_ID)) graph.unshift(organization);

  head.push({
    tag: 'script',
    attrs: { type: 'application/ld+json' },
    content: JSON.stringify({ '@context': 'https://schema.org', '@graph': graph }).replace(/</g, '\\u003c'),
  });
});
