/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import { createAppErrorRegistry, errorIdentity } from '../src/lib/app-error';

describe('app error registry', () => {
  test('deduplicates a config error across listeners and after dismissal', () => {
    const errors = createAppErrorRegistry();
    const identity = errorIdentity('config', 12);

    errors.show('duplicate profile name', identity);
    const first = errors.current;
    errors.show('duplicate profile name', identity);
    expect(errors.current).toEqual(first);

    errors.dismiss(first!.id);
    expect(errors.current).toBe(null);
    errors.show('duplicate profile name', identity);
    expect(errors.current).toBe(null);
  });

  test('surfaces the same message again for a new config revision', () => {
    const errors = createAppErrorRegistry();

    errors.show('duplicate profile name', errorIdentity('config', 12));
    const first = errors.current;
    errors.dismiss(first!.id);

    errors.show('duplicate profile name', errorIdentity('config', 13));
    expect(errors.current?.message).toBe('duplicate profile name');
    expect(errors.current?.id).not.toBe(first?.id);
  });
});
