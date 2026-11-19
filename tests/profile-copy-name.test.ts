/// <reference types="bun-types" />

import { expect, test } from 'bun:test';
import { nextProfileCopyName } from '../src/lib/profile-copy-name';

test('copy names remain unique and valid when the source fills the name limit', () => {
  const name = 'A'.repeat(80);
  const first = nextProfileCopyName(name, [name]);
  const second = nextProfileCopyName(name, [name, first.toUpperCase()]);

  expect(first).toBe(`${'A'.repeat(75)} copy`);
  expect(second).toBe(`${'A'.repeat(73)} copy 2`);
  expect(Array.from(second)).toHaveLength(80);
});
