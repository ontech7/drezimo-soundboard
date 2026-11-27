// What the Macro editor does to a time line: pure, on a copy, for any number of its clips and actions at once.
import type { Library, LibraryItem } from "./customPages";
import { clipLength, MAX_AT, MAX_TRACKS, type Timeline } from "./macros";

/** A clip or an action of the time line, by its place in its list */
export type Item = { kind: "clip" | "action"; index: number };

/** Moves land on tenths of a second, unless they land on an edge */
export const SNAP = 100;
/** A copied action alone comes this much after the one it copies, in milliseconds */
const ACTION_GAP = 500;

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));

export function contains(items: Item[], item: Item): boolean {
  return items.some((other) => other.kind === item.kind && other.index === item.index);
}

/** Shift+click: in or out of the selection */
export function toggled(items: Item[], item: Item): Item[] {
  return contains(items, item)
    ? items.filter((other) => !(other.kind === item.kind && other.index === item.index))
    : [...items, item];
}

export function soundOf(library: Library | null, id: string): LibraryItem | undefined {
  for (const soundboard of library?.soundboards ?? []) {
    const sound = soundboard.sounds.find((item) => item.id === id);
    if (sound) return sound;
  }
  return undefined;
}

/** Whether nothing plays on that track from `at` for `millis` */
export function isFree(timeline: Timeline, track: number, at: number, millis: number, library: Library | null): boolean {
  return !timeline.clips.some(
    (clip) => clip.track === track && clip.at < at + millis && at < clip.at + clipLength(clip, library).millis
  );
}

/** The first track free from `at` for `millis`, else the last */
export function freeTrack(timeline: Timeline, at: number, millis: number, library: Library | null): number {
  for (let track = 0; track < MAX_TRACKS; track += 1) {
    if (isFree(timeline, track, at, millis, library)) return track;
  }
  return MAX_TRACKS - 1;
}

export function removedAll(timeline: Timeline, items: Item[]): Timeline {
  return {
    clips: timeline.clips.filter((_, index) => !contains(items, { kind: "clip", index })),
    actions: timeline.actions.filter((_, index) => !contains(items, { kind: "action", index })),
  };
}

/**
 * The items moved as one block by `millis` and `tracks`: the block stops at the start of the time line, its end and
 * the first and last tracks, so that none of them moves alone
 */
export function movedAll(timeline: Timeline, items: Item[], millis: number, tracks: number): Timeline {
  const clips = items.filter((item) => item.kind === "clip").map((item) => timeline.clips[item.index]);
  const starts = [
    ...clips.map((clip) => clip.at),
    ...items.filter((item) => item.kind === "action").map((item) => timeline.actions[item.index].at),
  ];
  if (starts.length === 0) return timeline;
  const by = clamp(millis, -Math.min(...starts), MAX_AT - Math.max(...starts));
  const trackList = clips.map((clip) => clip.track);
  const down = trackList.length ? clamp(tracks, -Math.min(...trackList), MAX_TRACKS - 1 - Math.max(...trackList)) : 0;
  return {
    clips: timeline.clips.map((clip, index) =>
      contains(items, { kind: "clip", index }) ? { ...clip, at: clip.at + by, track: clip.track + down } : clip
    ),
    actions: timeline.actions.map((action, index) =>
      contains(items, { kind: "action", index }) ? { ...action, at: action.at + by } : action
    ),
  };
}

/**
 * Copies of the items right after the block they make, each clip on its own track if free there, else on the first
 * free one; the copies are the new selection
 */
export function duplicatedAll(
  timeline: Timeline,
  items: Item[],
  library: Library | null
): { timeline: Timeline; items: Item[] } {
  const clips = items.filter((item) => item.kind === "clip").map((item) => timeline.clips[item.index]);
  const actions = items.filter((item) => item.kind === "action").map((item) => timeline.actions[item.index]);
  const starts = [...clips.map((clip) => clip.at), ...actions.map((action) => action.at)];
  const ends = [...clips.map((clip) => clip.at + clipLength(clip, library).millis), ...actions.map((action) => action.at)];
  const span = Math.max(...ends) - Math.min(...starts) || ACTION_GAP;
  let next: Timeline = timeline;
  const copies: Item[] = [];
  for (const clip of clips) {
    const millis = clipLength(clip, library).millis;
    const at = Math.min(MAX_AT, clip.at + span);
    const track = isFree(next, clip.track, at, millis, library) ? clip.track : freeTrack(next, at, millis, library);
    copies.push({ kind: "clip", index: next.clips.length });
    next = { ...next, clips: [...next.clips, { ...clip, at, track }] };
  }
  for (const action of actions) {
    copies.push({ kind: "action", index: next.actions.length });
    next = { ...next, actions: [...next.actions, { ...action, at: Math.min(MAX_AT, action.at + span) }] };
  }
  return { timeline: next, items: copies };
}

/** Where moves land on: the start, the cursor, and the starts and ends of what is not moving */
export function edgesOf(timeline: Timeline, moving: Item[], library: Library | null, cursor: number): number[] {
  const clips = timeline.clips.flatMap((clip, index) =>
    contains(moving, { kind: "clip", index }) ? [] : [clip.at, clip.at + clipLength(clip, library).millis]
  );
  const actions = timeline.actions.flatMap((action, index) => (contains(moving, { kind: "action", index }) ? [] : [action.at]));
  return [0, cursor, ...clips, ...actions];
}

/**
 * Where something starting at `at` and `millis` long lands: its start or its end on the nearest edge within `reach`,
 * else on the tenths of a second
 */
export function snapped(at: number, millis: number, edges: number[], reach: number): number {
  let best: { at: number; off: number } | null = null;
  for (const edge of edges) {
    for (const start of [edge, edge - millis]) {
      const off = Math.abs(at - start);
      if (off <= reach && (!best || off < best.off)) best = { at: start, off };
    }
  }
  return Math.max(0, best ? best.at : Math.round(at / SNAP) * SNAP);
}

/**
 * The sounds a clip stops when it starts: Voicemod's own option on some sounds stops every other one then, so the clips
 * of other sounds playing at that moment are cut
 */
export function cutsOthers(timeline: Timeline, index: number, library: Library | null): string[] {
  const clip = timeline.clips[index];
  if (!soundOf(library, clip.id)?.stopsOthers) return [];
  const names = timeline.clips
    .filter(
      (other, i) =>
        i !== index &&
        other.id !== clip.id &&
        // At the same time, the runner starts them in their order: a later one starts after the stop
        (other.at < clip.at || (other.at === clip.at && i < index)) &&
        clip.at < other.at + clipLength(other, library).millis
    )
    .map((other) => soundOf(library, other.id)?.name ?? other.name);
  return [...new Set(names)];
}
