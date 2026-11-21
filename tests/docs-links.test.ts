import { describe, expect, test } from 'bun:test';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { docsLinks, docsUrl, providerDocsLinks, type DocsLink } from '../src/lib/docs-links';
import { PRODUCT_CONFIG } from '../src/lib/generated/bindings';

const DOCS_ROOT = join(import.meta.dir, '../docs/src/content/docs');

// Mirrors github-slugger, which Starlight uses for heading anchors.
function slug(heading: string): string {
  return heading
    .replace(/\\(.)/g, '$1')
    .replace(/[*`]/g, '')
    .trim()
    .toLowerCase()
    .replace(/[^\p{L}\p{M}\p{N}\p{Pc}\- ]/gu, '')
    .replace(/ /g, '-');
}

function pageSource(page: string): string | null {
  for (const extension of ['.md', '.mdx']) {
    const path = join(DOCS_ROOT, `${page}${extension}`);
    if (existsSync(path)) return readFileSync(path, 'utf8');
  }
  return null;
}

const links: [string, DocsLink][] = [
  ...Object.entries(docsLinks),
  ...Object.entries(providerDocsLinks).flatMap(([backend, provider]) =>
    Object.entries(provider)
      .filter((entry): entry is [string, DocsLink] => entry[1] !== null)
      .map(([kind, link]): [string, DocsLink] => [`${backend}.${kind}`, link]),
  ),
];

describe('docs links', () => {
  test('builds page and section URLs with trailing slashes', () => {
    expect(docsUrl()).toBe(`${PRODUCT_CONFIG.docsUrl}/`);
    expect(docsUrl('use/history', 'what-does-history-save')).toBe(
      `${PRODUCT_CONFIG.docsUrl}/use/history/#what-does-history-save`,
    );
  });

  test.each(links)('%s points to an existing page and heading', (_name, link) => {
    const url = new URL(link.url);
    const page = url.pathname.replace(/^\/|\/$/g, '');
    if (!page) return;
    const source = pageSource(page);
    expect(source).not.toBeNull();
    if (!url.hash) return;
    const anchors = (source ?? '')
      .split('\n')
      .filter((line) => /^#{2,6} /.test(line))
      .map((line) => slug(line.replace(/^#+ /, '')));
    expect(anchors).toContain(url.hash.slice(1));
  });
});
