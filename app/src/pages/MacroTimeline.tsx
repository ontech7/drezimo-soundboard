import {
  type CSSProperties,
  type KeyboardEvent,
  type MouseEvent as ReactMouseEvent,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
  useEffect,
  useRef,
  useState,
} from "react";
import { Segmented, Slider } from "../controls";
import { type Library, type Rgb, rgb, SOUND_COLOUR } from "../customPages";
import { type Texts, useTexts } from "../i18n";
import { PlusIcon, TrashIcon } from "../icons";
import {
  type Clip,
  clipLength,
  GUESSED_LENGTH,
  type Level,
  MAX_AT,
  MAX_CLIPS,
  MAX_STEPS,
  MAX_TRACKS,
  MIN_CUT,
  type MacroProgress,
  type Step,
  soundLength,
  type TimedStep,
  type Timeline,
  timelineLength,
} from "../macros";
import { useDismiss } from "../dismiss";
import { PadPicture } from "../PadPicture";
import type { PlaybackMode } from "../pressAgain";

/** Moves and cuts land on tenths of a second */
const SNAP = 100;
const LANE = 58;
const ACTION_LANE = 44;
/** The volume curve's height inside a clip, below its name */
const CURVE_TOP = 20;
const CURVE_HEIGHT = 30;
/** The time line always shows at least this much, in seconds, and a little past its end */
const MIN_SHOWN = 10;
const PAST_END = 2;
const ZOOMS = [30, 60, 100, 160];
/** As wide as the right-click menu draws, in pixels */
const MENU_WIDTH = 220;
/** As tall as its longest list can draw, in pixels (.mt-menu's max-height and a margin) */
const MENU_HEIGHT = 400;
/** A copied action comes this much after the one it copies, in milliseconds */
const ACTION_GAP = 500;

export type Selected = { kind: "clip"; index: number } | { kind: "action"; index: number } | null;

interface Described {
  title: string;
  icon: ReactNode;
  colour: Rgb;
}

/** An action the menus offer to add, made ready */
export interface ActionChoice {
  key: string;
  label: string;
  icon: ReactNode;
  colour: Rgb;
  step: Step;
}

interface Props {
  timeline: Timeline;
  library: Library | null;
  running: MacroProgress | null;
  /** Voicemod's bridge was found: clips can be cut and follow their own volume */
  bridge: boolean;
  selected: Selected;
  onSelect: (selected: Selected) => void;
  onChange: (timeline: Timeline) => void;
  /** Where a sound or an action added next goes, in milliseconds */
  cursor: number;
  onCursor: (at: number) => void;
  describeAction: (action: TimedStep) => Described;
  /** The right-click menu's: a sound to choose for that time and track, none for the first free one */
  onAddSound?: (at: number, track: number | null) => void;
  actionChoices?: ActionChoice[];
}

const snap = (millis: number) => Math.round(millis / SNAP) * SNAP;
const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));

function modeOf(library: Library | null, id: string): PlaybackMode | null {
  for (const soundboard of library?.soundboards ?? []) {
    const sound = soundboard.sounds.find((item) => item.id === id);
    if (sound) return sound.mode;
  }
  return null;
}

function pictureOf(library: Library | null, id: string): string | null {
  for (const soundboard of library?.soundboards ?? []) {
    const sound = soundboard.sounds.find((item) => item.id === id);
    if (sound) return sound.picture;
  }
  return null;
}

/** Clips of one sound that play at the same time: Voicemod plays it once, unless its mode stacks copies */
export function overlapsItself(timeline: Timeline, index: number, library: Library | null): boolean {
  const clip = timeline.clips[index];
  if (modeOf(library, clip.id) === "PlayOverlap") return false;
  const end = clip.at + clipLength(clip, library).millis;
  return timeline.clips.some(
    (other, i) =>
      i !== index && other.id === clip.id && other.at < end && clip.at < other.at + clipLength(other, library).millis
  );
}

/** The tracks shown: those in use and one free below them */
export function trackCount(timeline: Timeline): number {
  const used = Math.max(-1, ...timeline.clips.map((clip) => clip.track)) + 1;
  return clamp(used + 1, 2, MAX_TRACKS);
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

/** A copy of a clip or an action right after it: a clip on its own track if free there, else on the first free one */
export function duplicated(timeline: Timeline, target: NonNullable<Selected>, library: Library | null): Timeline {
  if (target.kind === "action") {
    const action = timeline.actions[target.index];
    return { ...timeline, actions: [...timeline.actions, { ...action, at: Math.min(MAX_AT, action.at + ACTION_GAP) }] };
  }
  const clip = timeline.clips[target.index];
  const millis = clipLength(clip, library).millis;
  const at = Math.min(MAX_AT, clip.at + millis);
  const track = isFree(timeline, clip.track, at, millis, library) ? clip.track : freeTrack(timeline, at, millis, library);
  return { ...timeline, clips: [...timeline.clips, { ...clip, at, track }] };
}

/** Where the round under way is, drawn at each frame from when it started */
function usePlayhead(running: MacroProgress | null): number | null {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!running) return;
    let frame = requestAnimationFrame(function tick() {
      setNow(Date.now());
      frame = requestAnimationFrame(tick);
    });
    return () => cancelAnimationFrame(frame);
  }, [running]);
  return running ? Math.max(0, now - running.roundStarted) : null;
}

/** The sounds of a macro on their tracks, its actions on a track of their own, along a time line */
export function MacroTimeline(props: Props) {
  const texts = useTexts();
  const { library, running, bridge, selected, onSelect, cursor, onCursor } = props;
  // While a drag goes on, the time line moves here and is saved once let go
  const [draft, setDraft] = useState<Timeline | null>(null);
  const [zoom, setZoom] = useState(60);
  const timeline = draft ?? props.timeline;
  const scroller = useRef<HTMLDivElement>(null);
  const playhead = usePlayhead(running);
  const tracks = trackCount(timeline);
  const shown = Math.max(MIN_SHOWN, timelineLength(timeline, library) + PAST_END, cursor / 1000 + PAST_END);
  const width = shown * zoom;
  const x = (millis: number) => (millis / 1000) * zoom;
  const millisAt = (pixels: number) => (pixels / zoom) * 1000;

  // The playhead stays in sight while the round plays
  useEffect(() => {
    const element = scroller.current;
    if (playhead === null || !element) return;
    const at = x(playhead);
    if (at < element.scrollLeft || at > element.scrollLeft + element.clientWidth - 40) element.scrollLeft = at - 40;
  });

  const drag = (event: ReactPointerEvent, move: (dx: number, dy: number, target: Element | null) => Timeline | null) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    const start = { x: event.clientX, y: event.clientY };
    let moved = false;
    let latest: Timeline | null = null;
    const onMove = (next: PointerEvent) => {
      const dx = next.clientX - start.x;
      const dy = next.clientY - start.y;
      if (!moved && Math.abs(dx) + Math.abs(dy) < 3) return;
      moved = true;
      latest = move(dx, dy, document.elementFromPoint(next.clientX, next.clientY));
      if (latest) setDraft(latest);
    };
    const onUp = (end: PointerEvent) => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onUp);
      setDraft(null);
      if (moved && latest && end.type === "pointerup") props.onChange(latest);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
  };

  const withClip = (index: number, change: (clip: Clip) => Clip): Timeline => ({
    ...props.timeline,
    clips: props.timeline.clips.map((clip, i) => (i === index ? change(clip) : clip)),
  });

  const keys = (event: KeyboardEvent) => {
    if (!selected || (event.key !== "Delete" && event.key !== "Backspace")) return;
    event.preventDefault();
    props.onChange(removed(props.timeline, selected));
    onSelect(null);
  };

  // The right-click menu: on a clip or an action what can be done to it, elsewhere on a track what can be added there
  const [menu, setMenu] = useState<MenuAt | null>(null);
  const root = useRef<HTMLDivElement>(null);
  const openMenu = (event: ReactMouseEvent<HTMLDivElement>) => {
    event.preventDefault();
    const element = event.target as HTMLElement;
    const lane = element.closest<HTMLElement>(".mt-lane");
    const box = root.current?.getBoundingClientRect();
    // Shown only, as in the tour: no menu
    if (!lane || !box || !props.onAddSound) return;
    const clip = element.closest<HTMLElement>(".mt-clip");
    const action = element.closest<HTMLElement>(".mt-action");
    const target: Selected = clip
      ? { kind: "clip", index: Number(clip.dataset.index) }
      : action
        ? { kind: "action", index: Number(action.dataset.index) }
        : null;
    if (target) onSelect(target);
    const at = clamp(snap(millisAt(event.clientX - event.currentTarget.getBoundingClientRect().left)), 0, MAX_AT);
    const track = lane.classList.contains("actions") ? null : Number(lane.dataset.track);
    // Upwards when the window has no room below for the longest menu, the actions' list
    const up = window.innerHeight - event.clientY < MENU_HEIGHT;
    setMenu({
      x: event.clientX - box.left,
      y: event.clientY - box.top,
      width: box.width,
      height: box.height,
      up,
      at,
      track,
      target,
    });
  };
  const full = {
    clips: props.timeline.clips.length >= MAX_CLIPS,
    actions: props.timeline.actions.length >= MAX_STEPS,
  };

  return (
    <div className="mt" onKeyDown={keys} ref={root}>
      <div className="mt-bar">
        <span className="muted">
          {running
            ? texts.stepOf(running.step, running.steps)
            : texts.timelineLength(texts.secondsOf(timelineLength(timeline, library)))}
        </span>
        <span className="spacer" />
        <Segmented
          label={texts.zoom}
          options={ZOOMS.map((value, i) => ({ value, label: texts.zoomLevels[i] }))}
          value={zoom}
          onChange={setZoom}
        />
      </div>
      <div className="mt-body">
        <div className="mt-heads" aria-hidden="true">
          <div className="mt-head ruler-head">{texts.tracks}</div>
          {Array.from({ length: tracks }, (_, track) => (
            <div key={track} className="mt-head" style={{ height: LANE }}>
              {texts.trackName(track + 1)}
            </div>
          ))}
          <div className="mt-head" style={{ height: ACTION_LANE }}>
            {texts.actionsTrack}
          </div>
        </div>
        <div className="mt-scroller" ref={scroller}>
          <div className="mt-canvas" style={{ width }} onContextMenu={openMenu}>
            <Ruler
              shown={shown}
              zoom={zoom}
              timeline={timeline}
              library={library}
              onPick={(pixels) => onCursor(clamp(snap(millisAt(pixels)), 0, MAX_AT))}
            />
            {Array.from({ length: tracks }, (_, track) => (
              <div key={track} className="mt-lane" data-track={track} style={{ height: LANE }}>
                {timeline.clips.map((clip, index) =>
                  clip.track === track ? (
                    <ClipBlock
                      key={index}
                      index={index}
                      clip={clip}
                      library={library}
                      zoom={zoom}
                      bridge={bridge}
                      selected={selected?.kind === "clip" && selected.index === index}
                      warned={overlapsItself(timeline, index, library)}
                      playing={
                        playhead !== null && clip.at <= playhead && playhead < clip.at + clipLength(clip, library).millis
                      }
                      onSelect={() => onSelect({ kind: "clip", index })}
                      onMove={(event) =>
                        drag(event, (dx, _, target) => {
                          const lane = target?.closest<HTMLElement>(".mt-lane");
                          const next = lane ? Number(lane.dataset.track) : clip.track;
                          return withClip(index, (old) => ({
                            ...old,
                            at: clamp(snap(old.at + millisAt(dx)), 0, MAX_AT),
                            track: Number.isFinite(next) ? next : old.track,
                          }));
                        })
                      }
                      onCut={(event) =>
                        drag(event, (dx) => {
                          const whole = soundLength(library, clip.id);
                          // A length not known yet is drawn as a guess: the cut stays within what is drawn
                          const full = whole === null ? null : Math.round(whole * 1000);
                          const now = clipLength(clip, library).millis;
                          const cut = clamp(snap(now + millisAt(dx)), MIN_CUT, full ?? GUESSED_LENGTH * 1000);
                          return withClip(index, (old) => ({ ...old, cut: full !== null && cut >= full ? null : cut }));
                        })
                      }
                      onLevel={(event, at, level) =>
                        drag(event, (dx, dy) => {
                          const levels = clip.volume ?? [];
                          const before = levels[at - 1]?.at ?? 0;
                          const after = levels[at + 1]?.at ?? clipLength(clip, library).millis;
                          const point: Level = {
                            at: clamp(snap(level.at + millisAt(dx)), before, after),
                            percent: Math.round(clamp(level.percent - (dy / CURVE_HEIGHT) * 100, 0, 100)),
                          };
                          return withClip(index, (old) => ({
                            ...old,
                            volume: (old.volume ?? []).map((other, i) => (i === at ? point : other)),
                          }));
                        })
                      }
                      onAddLevel={(at, percent) => {
                        const levels = [...(clip.volume ?? []), { at: snap(at), percent }].sort((a, b) => a.at - b.at);
                        props.onChange(withClip(index, (old) => ({ ...old, volume: levels })));
                      }}
                      onRemoveLevel={(at) =>
                        props.onChange(
                          withClip(index, (old) => ({ ...old, volume: (old.volume ?? []).filter((_, i) => i !== at) }))
                        )
                      }
                      texts={texts}
                    />
                  ) : null
                )}
              </div>
            ))}
            <div className="mt-lane actions" style={{ height: ACTION_LANE }}>
              {timeline.actions.map((action, index) => {
                const shown = props.describeAction(action);
                return (
                  <button
                    key={index}
                    type="button"
                    className={selected?.kind === "action" && selected.index === index ? "mt-action selected" : "mt-action"}
                    data-index={index}
                    style={{ left: x(action.at), "--tint": rgb(shown.colour) } as CSSProperties}
                    title={shown.title}
                    onClick={() => onSelect({ kind: "action", index })}
                    onPointerDown={(event) => {
                      onSelect({ kind: "action", index });
                      drag(event, (dx) => ({
                        ...props.timeline,
                        actions: props.timeline.actions.map((other, i) =>
                          i === index ? { ...other, at: clamp(snap(other.at + millisAt(dx)), 0, MAX_AT) } : other
                        ),
                      }));
                    }}
                  >
                    <i className="diamond" />
                    <span>{shown.title}</span>
                  </button>
                );
              })}
            </div>
            <div className="mt-cursor" style={{ left: x(cursor) }} aria-hidden="true" />
            {playhead !== null && <div className="mt-playhead" style={{ left: x(playhead) }} aria-hidden="true" />}
          </div>
        </div>
      </div>
      {menu && (
        <TimelineMenu
          menu={menu}
          full={full}
          actionChoices={props.actionChoices ?? []}
          onClose={() => setMenu(null)}
          onAddSound={() => props.onAddSound?.(menu.at, menu.track)}
          onAddAction={(step) => {
            const actions = [...props.timeline.actions, { ...step, at: menu.at } as TimedStep];
            props.onChange({ ...props.timeline, actions });
            onSelect({ kind: "action", index: props.timeline.actions.length });
          }}
          onDuplicate={(target) => {
            props.onChange(duplicated(props.timeline, target, library));
            onSelect(
              target.kind === "clip"
                ? { kind: "clip", index: props.timeline.clips.length }
                : { kind: "action", index: props.timeline.actions.length }
            );
          }}
          onDelete={(target) => {
            props.onChange(removed(props.timeline, target));
            onSelect(null);
          }}
        />
      )}
    </div>
  );
}

interface MenuAt {
  /** Where it opens, in the time line's own pixels, and how wide the time line is */
  x: number;
  y: number;
  width: number;
  height: number;
  /** Opens upwards from the point */
  up: boolean;
  /** The time and the track clicked: null on the actions' track */
  at: number;
  track: number | null;
  target: Selected;
}

/** What a right click offers: duplicate or delete what it is on, else add a sound or an action at that point */
function TimelineMenu(props: {
  menu: MenuAt;
  full: { clips: boolean; actions: boolean };
  actionChoices: ActionChoice[];
  onClose: () => void;
  onAddSound: () => void;
  onAddAction: (step: Step) => void;
  onDuplicate: (target: NonNullable<Selected>) => void;
  onDelete: (target: NonNullable<Selected>) => void;
}) {
  const texts = useTexts();
  const { menu, full, onClose } = props;
  const [actions, setActions] = useState(false);
  const element = useDismiss<HTMLDivElement>(true, onClose);
  const first = useRef<HTMLButtonElement>(null);
  useEffect(() => first.current?.focus(), [actions]);
  const run = (action: () => void) => () => {
    onClose();
    action();
  };
  const target = menu.target;
  const fullFor = target?.kind === "clip" ? full.clips : full.actions;
  return (
    <div
      className="menu mt-menu"
      role="menu"
      ref={element}
      // Towards the side with room, never past the time line's edge; .menu's own top and left give way
      style={{
        ...(menu.x > menu.width - MENU_WIDTH ? { left: "auto", right: menu.width - menu.x } : { left: menu.x }),
        ...(menu.up ? { top: "auto", bottom: menu.height - menu.y } : { top: menu.y }),
      }}
      onContextMenu={(event) => event.preventDefault()}
    >
      {target ? (
        <>
          <button
            type="button"
            role="menuitem"
            ref={first}
            disabled={fullFor}
            title={fullFor ? (target.kind === "clip" ? texts.tooManyClips : texts.tooManyActions) : undefined}
            onClick={run(() => props.onDuplicate(target))}
          >
            {texts.duplicate}
          </button>
          <button type="button" role="menuitem" className="danger" onClick={run(() => props.onDelete(target))}>
            {texts.deleteItem}
          </button>
        </>
      ) : actions ? (
        <>
          <span className="muted heading">{texts.addActionAt(texts.secondsOf(menu.at / 1000))}</span>
          {props.actionChoices.map((choice, i) => (
            <button
              key={choice.key}
              type="button"
              role="menuitem"
              ref={i === 0 ? first : undefined}
              className="with-icon"
              onClick={run(() => props.onAddAction(choice.step))}
            >
              <i className="dot" style={{ background: rgb(choice.colour) }} />
              {choice.label}
            </button>
          ))}
        </>
      ) : (
        <>
          <span className="muted heading">{texts.atSecond(texts.secondsOf(menu.at / 1000))}</span>
          <button
            type="button"
            role="menuitem"
            ref={first}
            disabled={full.clips}
            title={full.clips ? texts.tooManyClips : undefined}
            onClick={run(props.onAddSound)}
          >
            {texts.addSoundHere}
          </button>
          <button
            type="button"
            role="menuitem"
            disabled={full.actions}
            title={full.actions ? texts.tooManyActions : undefined}
            onClick={() => setActions(true)}
          >
            {texts.addActionHere}…
          </button>
        </>
      )}
    </div>
  );
}

/** Seconds along the top, and where two sounds or more play at once */
function Ruler(props: {
  shown: number;
  zoom: number;
  timeline: Timeline;
  library: Library | null;
  onPick: (pixels: number) => void;
}) {
  const texts = useTexts();
  const { shown, zoom } = props;
  const every = zoom < 45 ? 2 : 1;
  const ticks = Array.from({ length: Math.floor(shown / every) + 1 }, (_, i) => i * every);
  return (
    <div
      className="mt-ruler"
      onPointerDown={(event) => props.onPick(event.clientX - event.currentTarget.getBoundingClientRect().left)}
    >
      {ticks.map((second) => (
        <span key={second} className="tick" style={{ left: second * zoom }}>
          {texts.secondsShort(second)}
        </span>
      ))}
      {together(props.timeline, props.library).map((band, i) => (
        <span
          key={i}
          className="together"
          style={{ left: (band.start / 1000) * zoom, width: ((band.end - band.start) / 1000) * zoom }}
          title={texts.playTogether(band.count)}
        />
      ))}
    </div>
  );
}

/** The stretches where two sounds or more play at once */
export function together(timeline: Timeline, library: Library | null): { start: number; end: number; count: number }[] {
  const spans = timeline.clips.map((clip) => [clip.at, clip.at + clipLength(clip, library).millis] as const);
  const points = [...new Set(spans.flat())].sort((a, b) => a - b);
  const bands: { start: number; end: number; count: number }[] = [];
  for (let i = 0; i + 1 < points.length; i += 1) {
    const middle = (points[i] + points[i + 1]) / 2;
    const count = spans.filter(([start, end]) => start <= middle && middle < end).length;
    if (count < 2) continue;
    const last = bands[bands.length - 1];
    if (last && last.end === points[i] && last.count === count) last.end = points[i + 1];
    else bands.push({ start: points[i], end: points[i + 1], count });
  }
  return bands;
}

interface ClipProps {
  index: number;
  clip: Clip;
  library: Library | null;
  zoom: number;
  bridge: boolean;
  selected: boolean;
  /** Plays over itself: Voicemod starts it again instead */
  warned: boolean;
  playing: boolean;
  onSelect: () => void;
  onMove: (event: ReactPointerEvent) => void;
  onCut: (event: ReactPointerEvent) => void;
  onLevel: (event: ReactPointerEvent, index: number, level: Level) => void;
  onAddLevel: (at: number, percent: number) => void;
  onRemoveLevel: (index: number) => void;
  texts: Texts;
}

function ClipBlock(props: ClipProps) {
  const { clip, library, zoom, texts } = props;
  const length = clipLength(clip, library);
  const width = Math.max(14, (length.millis / 1000) * zoom);
  const x = (millis: number) => (millis / 1000) * zoom;
  const y = (percent: number) => CURVE_TOP + (1 - percent / 100) * CURVE_HEIGHT;
  const levels = clip.volume ?? [];
  const points = levels.map((level) => `${x(level.at)},${y(level.percent)}`);
  // Flat on from the last level to the end, as it plays
  const line = levels.length
    ? [`0,${y(levels[0].percent)}`, ...points, `${width},${y(levels[levels.length - 1].percent)}`]
    : [];
  const name = clip.name || texts.soundMissing;
  const classes = [
    "mt-clip",
    props.selected ? "selected" : "",
    props.playing ? "playing" : "",
    props.warned ? "warned" : "",
    length.known ? "" : "guessed",
  ];
  return (
    <div
      className={classes.join(" ")}
      style={{ left: x(clip.at), width, "--tint": rgb(SOUND_COLOUR) } as CSSProperties}
      role="button"
      tabIndex={0}
      data-index={props.index}
      aria-pressed={props.selected}
      aria-label={texts.clipLabel(name, texts.secondsOf(clip.at / 1000))}
      title={props.warned ? texts.overlapsItself : undefined}
      onPointerDown={(event) => {
        props.onSelect();
        props.onMove(event);
      }}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") props.onSelect();
      }}
      onDoubleClick={(event) => {
        if (!props.bridge) return;
        const box = event.currentTarget.getBoundingClientRect();
        const at = ((event.clientX - box.left) / zoom) * 1000;
        const percent = Math.round(clamp((1 - (event.clientY - box.top - CURVE_TOP) / CURVE_HEIGHT) * 100, 0, 100));
        props.onAddLevel(clamp(at, 0, length.millis), percent);
      }}
    >
      <span className="label">
        <PadPicture picture={pictureOf(library, clip.id)} />
        <span className="name">{name}</span>
        <span className="length">{length.known ? texts.secondsOf(length.millis / 1000) : "?"}</span>
      </span>
      {clip.cut != null && <span className="cut-mark" title={texts.cutAt(texts.secondsOf(clip.cut / 1000))} />}
      {line.length > 0 && (
        <svg className="curve" width={width} height={CURVE_TOP + CURVE_HEIGHT + 4} aria-hidden="true">
          <polyline points={line.join(" ")} />
          {levels.map((level, i) => (
            <circle
              key={i}
              cx={x(level.at)}
              cy={y(level.percent)}
              r={4.5}
              onPointerDown={(event) => {
                props.onSelect();
                props.onLevel(event, i, level);
              }}
              onDoubleClick={(event) => event.stopPropagation()}
              onContextMenu={(event) => {
                event.preventDefault();
                // A point's own menu: it goes, and the clip's menu does not open
                event.stopPropagation();
                props.onRemoveLevel(i);
              }}
            >
              <title>{texts.levelAt(level.percent, texts.secondsOf(level.at / 1000))}</title>
            </circle>
          ))}
        </svg>
      )}
      {props.bridge && (
        <span
          className="cut-handle"
          title={texts.dragToCut}
          onPointerDown={(event) => {
            props.onSelect();
            props.onCut(event);
          }}
        />
      )}
    </div>
  );
}

/** The time line without the selected clip or action */
export function removed(timeline: Timeline, selected: NonNullable<Selected>): Timeline {
  return selected.kind === "clip"
    ? { ...timeline, clips: timeline.clips.filter((_, i) => i !== selected.index) }
    : { ...timeline, actions: timeline.actions.filter((_, i) => i !== selected.index) };
}

type Shape = "own" | "steady" | "in" | "out" | "inOut";

/** The volume shapes a clip offers, at a top level */
function shaped(shape: Shape, top: number, millis: number): Level[] {
  const fade = Math.min(1500, Math.round(millis / 3 / SNAP) * SNAP);
  switch (shape) {
    case "own":
      return [];
    case "steady":
      return [{ at: 0, percent: top }];
    case "in":
      return [
        { at: 0, percent: 0 },
        { at: fade, percent: top },
      ];
    case "out":
      return [
        { at: millis - fade, percent: top },
        { at: millis, percent: 0 },
      ];
    case "inOut":
      return [
        { at: 0, percent: 0 },
        { at: fade, percent: top },
        { at: millis - fade, percent: top },
        { at: millis, percent: 0 },
      ];
  }
}

/** The shape the levels make, whatever the length of their fades: none for a curve drawn by hand */
function shapeOf(levels: Level[]): Shape | null {
  const pattern = levels.map((level) => (level.percent === 0 ? "0" : "top")).join(" ");
  const top = Math.max(0, ...levels.map((level) => level.percent));
  if (levels.some((level) => level.percent !== 0 && level.percent !== top)) return null;
  const shapes: Record<string, Shape> = { "": "own", top: "steady", "0 top": "in", "top 0": "out", "0 top top 0": "inOut" };
  const shape = shapes[pattern] ?? null;
  return shape === "in" || shape === "inOut" ? (levels[0].at === 0 ? shape : null) : shape;
}

/** The selected clip's settings, below the time line */
export function ClipInspector(props: {
  clip: Clip;
  library: Library | null;
  bridge: boolean;
  warned: boolean;
  onChange: (clip: Clip) => void;
  onChangeSound: () => void;
  onRemove: () => void;
}) {
  const texts = useTexts();
  const { clip, library, bridge } = props;
  const length = clipLength(clip, library);
  const levels = clip.volume ?? [];
  const top = levels.length ? Math.max(...levels.map((level) => level.percent)) : 100;
  const shape = shapeOf(levels);
  const whole = soundLength(library, clip.id);
  return (
    <div className="mt-inspector">
      <div className="identity">
        <PadPicture picture={pictureOf(library, clip.id)} />
        <div>
          <p className="title">{clip.name || texts.soundMissing}</p>
          <p className="muted">
            {texts.clipTimes(
              texts.secondsOf(clip.at / 1000),
              texts.secondsOf((clip.at + length.millis) / 1000),
              whole === null ? texts.unknownLength : texts.secondsOf(whole)
            )}
          </p>
        </div>
      </div>
      <span className="spacer" />
      {clip.cut != null && (
        <button type="button" className="link" onClick={() => props.onChange({ ...clip, cut: null })}>
          {texts.uncut}
        </button>
      )}
      <button type="button" className="button secondary" onClick={props.onChangeSound}>
        {texts.changeSound}
      </button>
      <button
        type="button"
        className="icon-button"
        aria-label={texts.removeStep}
        title={texts.removeStep}
        onClick={props.onRemove}
      >
        <TrashIcon />
      </button>
      <span className="row-break" />
      <div className="field shape-field">
        {texts.volumeShape}
        <Segmented
          label={texts.volumeShape}
          options={(["own", "steady", "in", "out", "inOut"] as Shape[]).map((value) => ({
            value,
            label: texts.volumeShapes[value],
          }))}
          value={shape ?? ("custom" as Shape)}
          disabled={!bridge}
          onChange={(next) => props.onChange({ ...clip, volume: shaped(next, top, length.millis) })}
        />
      </div>
      {levels.length > 0 && (
        <div className="field">
          {texts.volumeTop}
          <Slider
            label={texts.volumeTop}
            value={top}
            min={0}
            max={100}
            step={5}
            disabled={!bridge}
            onCommit={(next) => {
              const scale = top === 0 ? 0 : next / top;
              const volume = levels.map((level) => ({
                ...level,
                percent: top === 0 && level.percent === 0 ? next : Math.round(clamp(level.percent * scale, 0, 100)),
              }));
              props.onChange({ ...clip, volume });
            }}
          />
        </div>
      )}
      <p className="hint">
        {!bridge
          ? texts.timelineNeedsBridge
          : props.warned
            ? texts.overlapsItself
            : levels.length
              ? texts.curveHint
              : texts.clipHint}
      </p>
    </div>
  );
}

/** Adding to the time line: a sound or an action, where the cursor is */
export function AddToTimeline(props: {
  full: { clips: boolean; actions: boolean };
  onSound: () => void;
  actionMenu: ReactNode;
}) {
  const texts = useTexts();
  return (
    <div className="mt-add">
      <button
        type="button"
        className="add-step-button"
        disabled={props.full.clips}
        title={props.full.clips ? texts.tooManyClips : undefined}
        onClick={props.onSound}
      >
        <PlusIcon />
        {texts.addSound}
      </button>
      {props.actionMenu}
    </div>
  );
}
