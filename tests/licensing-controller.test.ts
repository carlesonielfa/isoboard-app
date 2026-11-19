import { expect, test } from 'bun:test';
import { createLicensingController } from '../src/lib/licensing.svelte';
import type { LicenseSnapshot } from '../src/lib/generated/bindings';

function snapshot(active: boolean): LicenseSnapshot {
  return {
    needsChoice: !active,
    evaluation: 'notStarted',
    startedAt: null,
    expiresAt: null,
    showExpiryNotice: false,
    error: null,
    commercial: {
      active,
      hasSavedKey: active,
      choiceCompleted: active,
      maskedKey: active ? '••••-test' : null,
      coveredMajor: active ? 1 : null,
      lastVerifiedAt: active ? 100 : null,
      supportExpiresAt: null,
      supportActive: false,
      checking: false,
      error: null,
    },
  };
}
function api(getLicenseStatus: () => Promise<LicenseSnapshot>) {
  return {
    getLicenseStatus,
    activateCommercialLicense: async () => snapshot(true),
    removeCommercialLicense: async () => snapshot(false),
    recheckCommercialLicense: async () => snapshot(true),
    continueWithoutLicenseKey: async () => snapshot(false),
    dismissEvaluationNotice: async () => snapshot(false),
  };
}
async function settle() {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

test('activation survives an older status response and a failed replacement', async () => {
  let resolveInitial!: (snapshot: LicenseSnapshot) => void;
  const commands = api(
    () =>
      new Promise((resolve) => {
        resolveInitial = resolve;
      }),
  );
  const controller = createLicensingController({
    isDesktop: () => true,
    stateFactory: (value) => value,
    focusTarget: new EventTarget(),
    listen: async () => () => {},
    commands,
  });
  const cleanup = controller.mount();
  expect(await controller.activate('test-key')).toBe(true);
  resolveInitial(snapshot(false));
  await settle();
  expect(controller.state.snapshot?.commercial.active).toBe(true);
  commands.activateCommercialLicense = async () => {
    throw new Error('Invalid key');
  };
  expect(await controller.activate('invalid-key')).toBe(false);
  expect(controller.state.snapshot?.commercial.active).toBe(true);
  expect(controller.state.error).toContain('Invalid key');
  cleanup();
});

test('a background change during status loading is fetched before waiting for the polling timer', async () => {
  let onChanged!: () => void;
  let resolveInitial!: (snapshot: LicenseSnapshot) => void;
  let calls = 0;
  let cleaned = false;
  const controller = createLicensingController({
    isDesktop: () => true,
    stateFactory: (value) => value,
    focusTarget: new EventTarget(),
    listen: async (listener) => {
      onChanged = () => listener({ event: 'license-changed-event', id: 1, payload: {} });
      return () => {
        cleaned = true;
      };
    },
    commands: api(() =>
      ++calls === 1
        ? new Promise((resolve) => {
            resolveInitial = resolve;
          })
        : Promise.resolve(snapshot(true)),
    ),
  });
  const cleanup = controller.mount();
  onChanged();
  resolveInitial(snapshot(false));
  await settle();
  expect(calls).toBe(2);
  expect(controller.state.snapshot?.commercial.active).toBe(true);
  cleanup();
  expect(cleaned).toBe(true);
});
