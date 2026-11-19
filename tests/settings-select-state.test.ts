import { describe, expect, test } from 'bun:test';
import {
  createSettingsSelectChangeController,
  createSettingsSelectValueChangeHandler,
} from '../src/lib/settings-select-state';

describe('settings select event routing', () => {
  test('routes B then A through the complete handler while the parent remains A', async () => {
    let disabled = false;
    let selectedValue = '';
    const authoritativeValue = 'a';
    const persistedValues: string[] = [];
    let resolveB: (() => void) | undefined;
    let resolveA: (() => void) | undefined;
    const controller = createSettingsSelectChangeController(
      () => authoritativeValue,
      (value) => {
        selectedValue = value;
      },
    );
    const handleValueChange = createSettingsSelectValueChangeHandler(
      () => disabled,
      (value) =>
        controller.select(value, (nextValue) => {
          persistedValues.push(nextValue);
          return new Promise<void>((resolve) => {
            if (nextValue === 'b') resolveB = resolve;
            else resolveA = resolve;
          });
        }),
    );

    controller.sync(authoritativeValue);
    const selectionB = handleValueChange('b');
    const selectionA = handleValueChange('a');

    expect(persistedValues).toEqual(['b', 'a']);
    expect(selectedValue).toBe('a');
    resolveB?.();
    await selectionB;
    expect(selectedValue).toBe('a');

    resolveA?.();
    await selectionA;
    expect(selectedValue).toBe('a');

    disabled = true;
    await handleValueChange('a');
    expect(persistedValues).toEqual(['b', 'a']);
  });
});

describe('settings select change controller', () => {
  test('sequences B then A even when the parent value is still A', async () => {
    let selectedValue = '';
    const authoritativeValue = 'a';
    const persistedValues: string[] = [];
    let resolveB: (() => void) | undefined;
    let resolveA: (() => void) | undefined;
    const controller = createSettingsSelectChangeController(
      () => authoritativeValue,
      (value) => {
        selectedValue = value;
      },
    );

    controller.sync(authoritativeValue);
    const selectionB = controller.select('b', () => {
      persistedValues.push('b');
      return new Promise<void>((resolve) => (resolveB = resolve));
    });
    const selectionA = controller.select('a', () => {
      persistedValues.push('a');
      return new Promise<void>((resolve) => (resolveA = resolve));
    });

    expect(selectedValue).toBe('a');
    expect(persistedValues).toEqual(['b', 'a']);
    resolveB?.();
    await selectionB;
    expect(selectedValue).toBe('a');

    resolveA?.();
    await selectionA;
    expect(selectedValue).toBe('a');
  });

  test('swallows rejected persistence and restores the latest authoritative value', async () => {
    let selectedValue = '';
    let authoritativeValue = 'saved';
    const controller = createSettingsSelectChangeController(
      () => authoritativeValue,
      (value) => {
        selectedValue = value;
      },
    );

    controller.sync(authoritativeValue);
    const pending = controller.select('rejected', async () => {
      await Promise.resolve();
      throw new Error('save failed');
    });
    authoritativeValue = 'server-value';
    controller.sync(authoritativeValue);

    await pending;
    expect(selectedValue).toBe('server-value');
  });
});
