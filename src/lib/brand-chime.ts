// The trailer's wordmark arpeggio: A5, D6 and A6 on sixteenth notes at 132 BPM.
const NOTES = [81, 86, 93];
const STEP_SECONDS = 60 / 132 / 4;
const NOTE_SECONDS = 1;
const NOTE_GAIN = 0.09;

let context: AudioContext | null = null;

function frequency(note: number): number {
  return 440 * 2 ** ((note - 69) / 12);
}

/** Plays the three-note brand chime. Each note is a sine with a faint second partial. */
export function playBrandChime(): void {
  if (typeof AudioContext === 'undefined') return;
  context ??= new AudioContext();
  const audio = context;
  void audio.resume().then(() => {
    const start = audio.currentTime + 0.02;
    NOTES.forEach((note, index) => {
      const at = start + index * STEP_SECONDS;
      const envelope = audio.createGain();
      envelope.gain.setValueAtTime(0, at);
      envelope.gain.linearRampToValueAtTime(NOTE_GAIN, at + 0.004);
      envelope.gain.setTargetAtTime(0, at + 0.004, 1 / 8);
      envelope.gain.setTargetAtTime(0, at + NOTE_SECONDS * 0.8, 0.05);
      const pan = new StereoPannerNode(audio, { pan: (index - 1) * 0.3 });
      envelope.connect(pan).connect(audio.destination);
      for (const [ratio, level] of [
        [1, 1],
        [2.003, 0.18],
      ]) {
        const partial = new GainNode(audio, { gain: level });
        const oscillator = new OscillatorNode(audio, { frequency: frequency(note) * ratio });
        oscillator.connect(partial).connect(envelope);
        oscillator.start(at);
        oscillator.stop(at + NOTE_SECONDS);
      }
    });
  });
}
