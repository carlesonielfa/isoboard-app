# Capture sounds

All cues are synthesized by `scripts/render-capture-sounds.mjs`; regenerate them
from the app root with `node scripts/render-capture-sounds.mjs`. Each cue is
rendered in isolation from simple synth voices with a short room effect.
Output is stereo 48 kHz, 16-bit PCM.

- `capture-start.wav`: shortcut click and 260 Hz tone.
- `capture-taken.wav`: release latch and 360 Hz tone, with a 120 ms smooth
  release after the 180 ms body so the tone settles without a hard cutoff.
- `capture-copied.wav`: ascending two-tone confirmation.
- `capture-failed.wav`: descending two-tone inference-error cue.

All cues share one normalization gain, preserving relative levels with 6 dB
of asset headroom. Playback applies the app's 0.5 volume multiplier.
