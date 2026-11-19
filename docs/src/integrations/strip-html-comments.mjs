import { readdir, readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';

// Markdown sources keep authoring markers (`<!-- os:macos -->`, TODO and
// PLACEHOLDER notes) that the Markdown processor passes through as HTML
// comments. This removes every comment from the built pages. Literal `<!--`
// in code samples is HTML-escaped, so it is not affected.
async function* htmlFiles(dir) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) yield* htmlFiles(path);
    else if (entry.name.endsWith('.html')) yield path;
  }
}

export default function stripHtmlComments() {
  return {
    name: 'strip-html-comments',
    hooks: {
      'astro:build:done': async ({ dir }) => {
        for await (const file of htmlFiles(fileURLToPath(dir))) {
          const html = await readFile(file, 'utf8');
          const stripped = html.replace(/<!--[\s\S]*?-->/g, '');
          if (stripped !== html) await writeFile(file, stripped);
        }
      },
    },
  };
}
