// A coin spinning on its vertical axis, modeled as a damped pendulum so it
// always comes to rest facing forward (a multiple of a full turn).
const RESTORING = 25;
const MAX_VELOCITY = 120;
const FLICK = 18;

export type Spin = { angle: number; velocity: number };

export const restingSpin: Spin = { angle: 0, velocity: 0 };

/**
 * Adds one flick. `offset` is where the click landed across the part, from
 * -1 (left edge) to 1 (right edge); its sign sets the direction and its size
 * the force. Repeated flicks accumulate.
 */
export function flick(spin: Spin, offset: number): Spin {
  const direction = offset < 0 ? -1 : 1;
  const force = FLICK * (0.8 + 0.3 * Math.min(Math.abs(offset), 1));
  const velocity = spin.velocity + direction * force;
  return {
    angle: spin.angle,
    velocity: Math.max(-MAX_VELOCITY, Math.min(MAX_VELOCITY, velocity)),
  };
}

/** Advances the spin by `seconds`. Damping rises as it slows so it settles with a short wobble. */
export function advance(spin: Spin, seconds: number): Spin {
  let { angle, velocity } = spin;
  const steps = Math.max(1, Math.ceil(seconds / (1 / 240)));
  const dt = seconds / steps;
  for (let i = 0; i < steps; i++) {
    const damping = 1 + 7 / (1 + (velocity * velocity) / 16);
    velocity += (-RESTORING * Math.sin(angle) - damping * velocity) * dt;
    angle += velocity * dt;
  }
  return { angle, velocity };
}

export function settled(spin: Spin): boolean {
  const turn = Math.PI * 2;
  const offset = spin.angle - Math.round(spin.angle / turn) * turn;
  return Math.abs(spin.velocity) < 0.02 && Math.abs(offset) < 0.002;
}
