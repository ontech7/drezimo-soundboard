const PENTATONIC = [0, 2, 4, 7, 9];
const ROOT_HZ = 196;

let context: AudioContext | null = null;
let output: AudioNode | null = null;
let voices: AudioScheduledSourceNode[] = [];

function audio() {
  if (!context) {
    context = new AudioContext();
    const limiter = context.createDynamicsCompressor();
    const volume = context.createGain();
    volume.gain.value = 0.5;
    limiter.connect(volume).connect(context.destination);
    output = limiter;
  }
  if (context.state === "suspended") void context.resume();
  return { ctx: context, out: output! };
}

function track(source: AudioScheduledSourceNode) {
  voices.push(source);
  source.onended = () => (voices = voices.filter((voice) => voice !== source));
  return source;
}

function tone(frequency: number, type: OscillatorType, length: number, peak = 0.4, slideTo?: number) {
  const { ctx, out } = audio();
  const now = ctx.currentTime;
  const oscillator = ctx.createOscillator();
  const envelope = ctx.createGain();
  oscillator.type = type;
  oscillator.frequency.setValueAtTime(frequency, now);
  if (slideTo) oscillator.frequency.exponentialRampToValueAtTime(slideTo, now + length);
  envelope.gain.setValueAtTime(0.0001, now);
  envelope.gain.exponentialRampToValueAtTime(peak, now + 0.006);
  envelope.gain.exponentialRampToValueAtTime(0.0001, now + length);
  oscillator.connect(envelope).connect(out);
  track(oscillator).start(now);
  oscillator.stop(now + length + 0.02);
}

function noise(length: number, filterHz: number, peak = 0.3) {
  const { ctx, out } = audio();
  const now = ctx.currentTime;
  const buffer = ctx.createBuffer(1, Math.ceil(ctx.sampleRate * length), ctx.sampleRate);
  const samples = buffer.getChannelData(0);
  for (let i = 0; i < samples.length; i++) samples[i] = Math.random() * 2 - 1;
  const source = ctx.createBufferSource();
  const filter = ctx.createBiquadFilter();
  const envelope = ctx.createGain();
  source.buffer = buffer;
  filter.type = "bandpass";
  filter.frequency.value = filterHz;
  envelope.gain.setValueAtTime(peak, now);
  envelope.gain.exponentialRampToValueAtTime(0.0001, now + length);
  source.connect(filter).connect(envelope).connect(out);
  track(source).start(now);
}

const TIMBRES: OscillatorType[] = ["triangle", "square", "sine"];

/** Pads play notes, low at the front and high at the back; the function keys their own effects. */
export function playKey(n: number, page: number) {
  if (n <= 36) {
    const row = Math.floor((n - 1) / 6);
    const column = (n - 1) % 6;
    const step = column + 2 * (5 - row);
    const semitones = 12 * Math.floor(step / 5) + PENTATONIC[step % 5];
    const frequency = ROOT_HZ * 2 ** (semitones / 12);
    const timbre = TIMBRES[page % TIMBRES.length];
    tone(frequency, timbre, 0.7, timbre === "square" ? 0.12 : 0.35);
    tone(frequency * 2, "sine", 0.25, 0.08);
    return;
  }
  switch (n) {
    case 37:
      return tone(660, "square", 0.12, 0.1, 330);
    case 38:
      return tone(330, "square", 0.12, 0.1, 660);
    case 39:
      stopAll();
      return tone(110, "sine", 0.35, 0.6, 40);
    case 40:
      return noise(0.08, 2400, 0.4);
    case 41:
      tone(220, "sawtooth", 0.5, 0.12, 880);
      return tone(223, "sawtooth", 0.5, 0.12, 440);
    case 42:
      return [880, 1320, 1760].forEach((frequency, i) => tone(frequency, "sine", 0.9 - i * 0.2, 0.12));
  }
}

export function stopAll() {
  for (const voice of voices) voice.stop();
  voices = [];
}
