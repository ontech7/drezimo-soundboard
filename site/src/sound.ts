/** Recordings from Freesound, all CC0: public/sounds/CREDITS.txt says whose each one is. */
export const SOUNDS = [
  "airhorn",
  "applause",
  "drum-roll",
  "rimshot",
  "sad-trombone",
  "bruh",
  "boing",
  "laser",
  "ding",
  "wow",
  "honk",
  "crickets",
  "victory",
  "fail",
  "bleep",
  "record-scratch",
  "evil-laugh",
  "ka-ching",
] as const;

const EFFECTS = ["click", "switch"] as const;
type Name = (typeof SOUNDS)[number] | (typeof EFFECTS)[number];

const MAX_VOICES = 8;

/** Which sound a pad plays: each page starts six sounds further on, so turning the knob changes them. */
export function padSound(n: number, page: number): number {
  return (n - 1 + page * 6) % SOUNDS.length;
}

let context: AudioContext | null = null;
let output: AudioNode | null = null;
let voices: AudioBufferSourceNode[] = [];
let stops = 0;
// What stops the time lines playing, with every other sound
const timelines = new Set<() => void>();
const fetched = new Map<Name, Promise<ArrayBuffer | null>>();
const decoded = new Map<Name, Promise<AudioBuffer | null>>();

function audio() {
  // A browser only lets sound start after a click, a tap or a typed key; Esc alone does not count.
  if (!context && navigator.userActivation?.hasBeenActive === false) return null;
  if (!context) {
    context = new AudioContext();
    const limiter = context.createDynamicsCompressor();
    const volume = context.createGain();
    volume.gain.value = 0.8;
    limiter.connect(volume).connect(context.destination);
    output = limiter;
  }
  if (context.state === "suspended") void context.resume();
  return { ctx: context, out: output! };
}

function download(name: Name) {
  let bytes = fetched.get(name);
  if (!bytes) {
    bytes = fetch(`${import.meta.env.BASE_URL}sounds/${name}.mp3`)
      .then((response) => (response.ok ? response.arrayBuffer() : null))
      .catch(() => null);
    fetched.set(name, bytes);
  }
  return bytes;
}

/** Decodes a recording once; a failed download or decode is forgotten, so the next press tries again. */
function buffer(name: Name, ctx: AudioContext) {
  let sound = decoded.get(name);
  if (!sound) {
    sound = download(name)
      .then((bytes) => {
        fetched.delete(name);
        return bytes ? ctx.decodeAudioData(bytes) : null;
      })
      .catch(() => null)
      .then((result) => {
        if (!result) decoded.delete(name);
        return result;
      });
    decoded.set(name, sound);
  }
  return sound;
}

/** Fetches every recording once the page is idle, so the first press does not wait for the network. */
export function prefetchSounds() {
  const start = () => [...SOUNDS, ...EFFECTS].forEach(download);
  if ("requestIdleCallback" in window) requestIdleCallback(start, { timeout: 3000 });
  else setTimeout(start, 1500);
}

function play(name: Name) {
  const engine = audio();
  if (!engine) return;
  const { ctx, out } = engine;
  const stopsBefore = stops;
  void buffer(name, ctx).then((sound) => {
    if (!sound || stops !== stopsBefore) return;
    const source = ctx.createBufferSource();
    source.buffer = sound;
    source.connect(out);
    source.onended = () => (voices = voices.filter((voice) => voice !== source));
    if (voices.length >= MAX_VOICES) voices.shift()!.stop();
    voices.push(source);
    source.start();
  });
}

export function playSound(index: number) {
  play(SOUNDS[index % SOUNDS.length]);
}

/** Pads play their sound; the page keys click, STOP silences everything, the toggles switch. */
export function playKey(n: number, page: number) {
  if (n <= 36) return playSound(padSound(n, page));
  if (n === 37 || n === 38) return play("click");
  if (n === 39) {
    stopAll();
    return play("click");
  }
  play("switch");
}

function stopAll() {
  stops++;
  for (const stop of timelines) stop();
  for (const voice of voices) voice.stop();
  voices = [];
}

export interface Clip {
  sound: (typeof SOUNDS)[number];
  /** Its start and, when cut, how long it plays, in seconds */
  at: number;
  cut?: number;
  /** Rising over its first third and falling over its last */
  fade?: boolean;
}

export interface Playing {
  /** Seconds since the time line started, on the audio clock */
  elapsed: () => number;
  stop: () => void;
}

/** Plays a macro's clips at their times, over each other; null before the first click, tap or typed key. */
export function playTimeline(clips: Clip[]): Playing | null {
  const engine = audio();
  if (!engine) return null;
  const { ctx, out } = engine;
  const started = ctx.currentTime + 0.08;
  const sources: AudioBufferSourceNode[] = [];
  let stopped = false;
  const stop = () => {
    stopped = true;
    timelines.delete(stop);
    for (const source of sources) source.stop();
  };
  timelines.add(stop);
  for (const clip of clips) {
    void buffer(clip.sound, ctx).then((sound) => {
      if (!sound || stopped) return;
      const start = started + clip.at;
      const length = Math.min(clip.cut ?? sound.duration, sound.duration);
      // Decoded late, it starts where it would be by now
      const when = Math.max(start, ctx.currentTime);
      if (when - start >= length) return;
      const gain = ctx.createGain();
      if (clip.fade) {
        gain.gain.setValueAtTime(0, start);
        gain.gain.linearRampToValueAtTime(1, start + length * 0.3);
        gain.gain.setValueAtTime(1, start + length * 0.7);
        gain.gain.linearRampToValueAtTime(0, start + length);
      }
      const source = ctx.createBufferSource();
      source.buffer = sound;
      source.connect(gain).connect(out);
      source.start(when, when - start, length - (when - start));
      sources.push(source);
    });
  }
  return { elapsed: () => ctx.currentTime - started, stop };
}
