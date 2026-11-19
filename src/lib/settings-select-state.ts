export type SettingsSelectChangeHandler = (value: string) => void | Promise<void>;

/**
 * Creates the value-change handler passed to the Bits UI select.
 *
 * The selected value is intentionally the only value delegated when enabled,
 * so a selection that returns to the parent's current value still reaches
 * persistence.
 */
export function createSettingsSelectValueChangeHandler(
  getDisabled: () => boolean,
  onValueChange: SettingsSelectChangeHandler,
): (nextValue: string) => Promise<void> {
  return async (nextValue: string): Promise<void> => {
    if (!getDisabled()) await onValueChange(nextValue);
  };
}

export type SettingsSelectChangeController = {
  sync(authoritativeValue: string): void;
  select(nextValue: string, onValueChange: SettingsSelectChangeHandler): Promise<void>;
};

/**
 * Keeps the value shown by Bits UI aligned with the latest parent-owned value
 * while allowing a selection to remain visible until its persistence callback
 * settles.
 */
export function createSettingsSelectChangeController(
  getAuthoritativeValue: () => string,
  setSelectedValue: (value: string) => void,
): SettingsSelectChangeController {
  let latestSequence = 0;

  return {
    sync(nextAuthoritativeValue: string): void {
      setSelectedValue(nextAuthoritativeValue);
    },

    async select(nextValue: string, onValueChange: SettingsSelectChangeHandler): Promise<void> {
      const sequence = ++latestSequence;
      setSelectedValue(nextValue);

      try {
        await onValueChange(nextValue);
      } catch {
        // Persistence callbacks own error reporting; always restore the visible value below.
      }

      if (sequence === latestSequence) setSelectedValue(getAuthoritativeValue());
    },
  };
}
