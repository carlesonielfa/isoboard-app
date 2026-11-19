import { writeFile } from 'node:fs/promises';

const RATE = 48_000;
const FRAMES = RATE * 2;
const TAU = Math.PI * 2;
const ease = (value) => {
  const x = Math.max(0, Math.min(1, value));
  return x * x * (3 - 2 * x);
};
const hz = (midi) => 440 * 2 ** ((midi - 69) / 12);

function renderCue(render) {
  const track = [new Float32Array(FRAMES), new Float32Array(FRAMES)];
  let seed = 73919;
  const random = () => {
    seed ^= seed << 13;
    seed ^= seed >>> 17;
    seed ^= seed << 5;
    return ((seed >>> 0) / 4294967296) * 2 - 1;
  };

  function add(start, duration, voice, gain, pan) {
    const offset = Math.round(start * RATE);
    const count = Math.round(duration * RATE);
    for (let i = 0; i < count && offset + i < FRAMES; i++) {
      const value = voice(i / RATE) * gain;
      track[0][offset + i] += value * Math.sqrt((1 - pan) / 2);
      track[1][offset + i] += value * Math.sqrt((1 + pan) / 2);
    }
  }

  function click(start, gain, pan = 0) {
    let low = 0;
    add(
      start,
      0.095,
      (t) => {
        low += 0.35 * (random() - low);
        return (0.5 * low + 0.55 * Math.sin(TAU * 680 * t)) * Math.exp(-t * 70) * ease(t / 0.002);
      },
      gain,
      pan,
    );
  }

  function bell(start, frequency, gain, pan, duration, release = 0) {
    add(
      start,
      duration,
      (t) =>
        (Math.sin(TAU * frequency * t) + 0.23 * Math.sin(TAU * frequency * 2.003 * t)) *
        Math.exp(-t * 8) *
        ease(t / 0.004) *
        (release ? ease((duration - t) / release) : 1),
      gain,
      pan,
    );
  }

  render({ click, bell });

  // Apply the promo's room taps after the dry voices, including their releases.
  for (const [delay, gain, cross] of [
    [0.071, 0.32, true],
    [0.113, 0.25, false],
    [0.197, 0.19, true],
    [0.307, 0.13, false],
    [0.431, 0.09, true],
  ]) {
    const offset = Math.floor(delay * RATE);
    for (let i = FRAMES - 1; i >= offset; i--) {
      track[0][i] += track[cross ? 1 : 0][i - offset] * gain * 0.2;
      track[1][i] += track[cross ? 0 : 1][i - offset] * gain * 0.2;
    }
  }
  return track;
}

function wav(track, gain) {
  const buffer = Buffer.alloc(44 + FRAMES * 4);
  buffer.write('RIFF', 0);
  buffer.writeUInt32LE(buffer.length - 8, 4);
  buffer.write('WAVEfmt ', 8);
  buffer.writeUInt32LE(16, 16);
  buffer.writeUInt16LE(1, 20);
  buffer.writeUInt16LE(2, 22);
  buffer.writeUInt32LE(RATE, 24);
  buffer.writeUInt32LE(RATE * 4, 28);
  buffer.writeUInt16LE(4, 32);
  buffer.writeUInt16LE(16, 34);
  buffer.write('data', 36);
  buffer.writeUInt32LE(FRAMES * 4, 40);
  for (let i = 0; i < FRAMES; i++) {
    for (let channel = 0; channel < 2; channel++) {
      buffer.writeInt16LE(Math.round(track[channel][i] * gain * 32767), 44 + i * 4 + channel * 2);
    }
  }
  return buffer;
}

const cues = {
  'capture-start': renderCue(({ click, bell }) => {
    click(0, 0.32);
    bell(0, 260, 0.13, 0, 0.25);
  }),
  'capture-taken': renderCue(({ click, bell }) => {
    click(0, 0.28, 0.25);
    // Keep the 180 ms body, then release smoothly instead of cutting off the tone.
    bell(0, 360, 0.14, 0.25, 0.3, 0.12);
  }),
  'capture-copied': renderCue(({ bell }) => {
    bell(0, hz(81), 0.1, 0, 0.45);
    bell(0.18, hz(86), 0.15, 0, 0.55);
  }),
  // Descending counterpart of the copied cue, lower and shorter.
  'capture-failed': renderCue(({ bell }) => {
    bell(0, hz(72), 0.13, 0, 0.22, 0.08);
    bell(0.16, hz(67), 0.15, 0, 0.4, 0.15);
  }),
};

let peak = 0;
for (const track of Object.values(cues)) {
  for (const channel of track) {
    for (const sample of channel) peak = Math.max(peak, Math.abs(sample));
  }
}
const gain = 0.5 / peak;
for (const [name, track] of Object.entries(cues)) {
  await writeFile(
    new URL(`../src-tauri/assets/audio/${name}.wav`, import.meta.url),
    wav(track, gain),
  );
}
