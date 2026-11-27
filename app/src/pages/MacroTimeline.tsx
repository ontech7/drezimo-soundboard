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
import { PlusIcon, RedoIcon, TrashIcon, UndoIcon } from "../icons";
import {
  contains,
  cutsOthers,
  duplicatedAll,
  edgesOf,
  type Item,
  movedAll,
  removedAll,
  SNAP,
  snapped,
  soundOf,
  toggled,
} from "../macroEdit";
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
/** A move lands on an edge this close, in pixels whatever the zoom */
const SNAP_REACH = 8;
/** The arrow keys move this much, with Shift ten times as much, in milliseconds */
const NUDGE = 100;

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
  /** What is selected: one item, or several with Shift+click */
  selection: Item[];
  onSelection: (items: Item[]) => void;
  onChange: (timeline: Timeline) => void;
  /** Where a sound or an action added next goes, in milliseconds */
  cursor: number;
  onCursor: (at: number) => void;
  describeAction: (action: TimedStep) => Described;
  /** The right-click menu's: a sound to choose for that time and track, none for the first free one */
  onAddSound?: (at: number, track: number | null) => void;
  actionChoices?: ActionChoice[];
  /** Undo and redo, where the editor keeps a history */
  history?: { canUndo: boolean; canRedo: boolean; undo: () => void; redo: () => void };
}

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));

/** Clips of one sound that play at the same time: Voicemod plays it once, unless its mode stacks copies */
export function overlapsItself(timeline: Timeline, index: number, library: Library | null): boolean {
  const clip = timeline.clips[index];
  if (soundOf(library, clip.id)?.mode === "PlayOverlap") return false;
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
  const { library, running, bridge, selection, onSelection, cursor, onCursor, history } = props;
  // While a drag goes on, the time line moves here and is saved once let go
  const [draft, setDraft] = useState<Timeline | null>(null);
  const [zoom, setZoom] = useState(60);
  const timeline = draft ?? props.timeline;
  const root = useRef<HTMLDivElement>(null);
  const scroller = useRef<HTMLDivElement>(null);
  const playhead = usePlayhead(running);
  const tracks = trackCount(timeline);
  const shown = Math.max(MIN_SHOWN, timelineLength(timeline, library) + PAST_END, cursor / 1000 + PAST_END);
  const width = shown * zoom;
  const x = (millis: number) => (millis / 1000) * zoom;
  const millisAt = (pixels: number) => (pixels / zoom) * 1000;
  const reach = millisAt(SNAP_REACH);

  // The playhead stays in sight while the round plays
  useEffect(() => {
    const element = scroller.current;
    if (playhead === null || !element) return;
    const at = x(playhead);
    if (at < element.scrollLeft || at > element.scrollLeft + element.clientWidth - 40) element.scrollLeft = at - 40;
  });

  /** `still`: let go without moving, as a plain click */
  const drag = (
    event: ReactPointerEvent,
    move: (dx: number, dy: number, target: Element | null) => Timeline | null,
    still?: () => void
  ) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    // The keys (Delete, arrows, undo) come to the time line from now on
    root.current?.focus({ preventScroll: true });
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
      if (end.type !== "pointerup") return;
      if (moved && latest) props.onChange(latest);
      if (!moved) still?.();
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
  };

  /**
   * A clip or an action grabbed: Shift adds it to the selection or takes it out; otherwise it moves, with the others
   * selected when it is one of them, its start or end landing on the edges near it
   */
  const grab = (event: ReactPointerEvent, item: Item) => {
    if (event.button !== 0) return;
    if (event.shiftKey) {
      event.preventDefault();
      root.current?.focus({ preventScroll: true });
      onSelection(toggled(selection, item));
      return;
    }
    const group = contains(selection, item) && selection.length > 1 ? selection : [item];
    if (group.length === 1) onSelection(group);
    const anchor = item.kind === "clip" ? props.timeline.clips[item.index] : props.timeline.actions[item.index];
    const millis = item.kind === "clip" ? clipLength(props.timeline.clips[item.index], library).millis : 0;
    const edges = edgesOf(props.timeline, group, library, cursor);
    drag(
      event,
      (dx, _, target) => {
        const at = snapped(anchor.at + millisAt(dx), millis, edges, reach);
        let tracks = 0;
        if (item.kind === "clip") {
          const lane = target?.closest<HTMLElement>(".mt-lane[data-track]");
          if (lane) tracks = Number(lane.dataset.track) - props.timeline.clips[item.index].track;
        }
        return movedAll(props.timeline, group, at - anchor.at, tracks);
      },
      // One of a group clicked without moving: that one alone
      () => group.length > 1 && onSelection([item])
    );
  };

  const withClip = (index: number, change: (clip: Clip) => Clip): Timeline => ({
    ...props.timeline,
    clips: props.timeline.clips.map((clip, i) => (i === index ? change(clip) : clip)),
  });

  const keys = (event: KeyboardEvent) => {
    if (selection.length === 0) return;
    if (event.key === "Escape") {
      onSelection([]);
      return;
    }
    if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      props.onChange(removedAll(props.timeline, selection));
      onSelection([]);
      root.current?.focus({ preventScroll: true });
      return;
    }
    const step = event.shiftKey ? NUDGE * 10 : NUDGE;
    const moves: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -1],
      ArrowDown: [0, 1],
    };
    const move = moves[event.key];
    if (!move) return;
    event.preventDefault();
    props.onChange(movedAll(props.timeline, selection, move[0], move[1]));
    // A clip moved to another track is drawn anew: the keys stay with the time line
    root.current?.focus({ preventScroll: true });
  };

  // The right-click menu: on a clip or an action what can be done to it, elsewhere on a track what can be added there
  const [menu, setMenu] = useState<MenuAt | null>(null);
  const openMenu = (event: ReactMouseEvent<HTMLDivElement>) => {
    event.preventDefault();
    const element = event.target as HTMLElement;
    const lane = element.closest<HTMLElement>(".mt-lane");
    const box = root.current?.getBoundingClientRect();
    // Shown only, as in the tour: no menu
    if (!lane || !box || !props.onAddSound) return;
    const clip = element.closest<HTMLElement>(".mt-clip");
    const action = element.closest<HTMLElement>(".mt-action");
    const item: Item | null = clip
      ? { kind: "clip", index: Number(clip.dataset.index) }
      : action
        ? { kind: "action", index: Number(action.dataset.index) }
        : null;
    // On one of several selected, the menu acts on them all
    const targets = item ? (contains(selection, item) ? selection : [item]) : null;
    if (targets) onSelection(targets);
    const at = clamp(snapped(millisAt(event.clientX - event.currentTarget.getBoundingClientRect().left), 0, [], 0), 0, MAX_AT);
    const track = lane.dataset.track === undefined ? null : Number(lane.dataset.track);
    // Upwards when there is no room below for the longest menu, the actions' list, within the window or the frame
    // that clips the time line, as the guide's; as tall as the room on that side allows
    const frame = root.current?.closest("[data-menu-frame]")?.getBoundingClientRect();
    const below = (frame?.bottom ?? window.innerHeight) - event.clientY;
    const above = event.clientY - (frame?.top ?? 0);
    const up = below < MENU_HEIGHT && above > below;
    setMenu({
      x: event.clientX - box.left,
      y: event.clientY - box.top,
      width: box.width,
      height: box.height,
      up,
      room: (up ? above : below) - 8,
      at,
      track,
      targets,
    });
  };
  // What can still be added before the macro is full
  const room = {
    clips: MAX_CLIPS - props.timeline.clips.length,
    actions: MAX_STEPS - props.timeline.actions.length,
  };
  // A menu left open over a time line changed meanwhile would act on what is no longer there
  useEffect(() => setMenu(null), [props.timeline]);
  // A button that disables itself, or a clip that moves to another track, would take the keys' focus with it
  const keepKeys = (action: () => void) => () => {
    action();
    root.current?.focus({ preventScroll: true });
  };
  const duplicate = (items: Item[]) => {
    const copied = duplicatedAll(props.timeline, items, library);
    props.onChange(copied.timeline);
    onSelection(copied.items);
  };

  return (
    // Focusable, for the keys: the clips themselves take the focus only from the keyboard
    <div className="mt" onKeyDown={keys} ref={root} tabIndex={-1}>
      <div className="mt-bar">
        <span className="muted">
          {running
            ? texts.stepOf(running.step, running.steps)
            : selection.length > 1
              ? texts.selectedCount(selection.length)
              : texts.timelineLength(texts.secondsOf(timelineLength(timeline, library)))}
        </span>
        <span className="spacer" />
        {history && (
          <>
            <button
              type="button"
              className="icon-button small"
              aria-label={texts.undo}
              title={texts.undo}
              disabled={!history.canUndo}
              onClick={keepKeys(history.undo)}
            >
              <UndoIcon />
            </button>
            <button
              type="button"
              className="icon-button small"
              aria-label={texts.redo}
              title={texts.redo}
              disabled={!history.canRedo}
              onClick={keepKeys(history.redo)}
            >
              <RedoIcon />
            </button>
          </>
        )}
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
              onPick={(pixels) => onCursor(clamp(snapped(millisAt(pixels), 0, [], 0), 0, MAX_AT))}
            />
            {Array.from({ length: tracks }, (_, track) => (
              <div
                key={track}
                className="mt-lane"
                data-track={track}
                style={{ height: LANE }}
                // A click on the empty track lets go of the selection
                onPointerDown={(event) => event.target === event.currentTarget && event.button === 0 && onSelection([])}
              >
                {timeline.clips.map((clip, index) =>
                  clip.track === track ? (
                    <ClipBlock
                      key={index}
                      index={index}
                      clip={clip}
                      library={library}
                      zoom={zoom}
                      bridge={bridge}
                      selected={contains(selection, { kind: "clip", index })}
                      warned={overlapsItself(timeline, index, library)}
                      cuts={cutsOthers(timeline, index, library)}
                      playing={
                        playhead !== null && clip.at <= playhead && playhead < clip.at + clipLength(clip, library).millis
                      }
                      onSelect={() => onSelection([{ kind: "clip", index }])}
                      onGrab={(event) => grab(event, { kind: "clip", index })}
                      onCut={(event) => {
                        onSelection([{ kind: "clip", index }]);
                        const edges = edgesOf(props.timeline, [{ kind: "clip", index }], library, cursor);
                        drag(event, (dx) => {
                          const whole = soundLength(library, clip.id);
                          // A length not known yet is drawn as a guess: the cut stays within what is drawn
                          const full = whole === null ? null : Math.round(whole * 1000);
                          const now = clipLength(clip, library).millis;
                          const end = snapped(clip.at + now + millisAt(dx), 0, edges, reach);
                          const cut = clamp(end - clip.at, MIN_CUT, full ?? GUESSED_LENGTH * 1000);
                          return withClip(index, (old) => ({ ...old, cut: full !== null && cut >= full ? null : cut }));
                        });
                      }}
                      onLevel={(event, at, level) => {
                        onSelection([{ kind: "clip", index }]);
                        drag(event, (dx, dy) => {
                          const levels = clip.volume ?? [];
                          const before = levels[at - 1]?.at ?? 0;
                          const after = levels[at + 1]?.at ?? clipLength(clip, library).millis;
                          const point: Level = {
                            at: clamp(snapped(level.at + millisAt(dx), 0, [], 0), before, after),
                            percent: Math.round(clamp(level.percent - (dy / CURVE_HEIGHT) * 100, 0, 100)),
                          };
                          return withClip(index, (old) => ({
                            ...old,
                            volume: (old.volume ?? []).map((other, i) => (i === at ? point : other)),
                          }));
                        });
                      }}
                      onAddLevel={(at, percent) => {
                        const levels = [...(clip.volume ?? []), { at: snapped(at, 0, [], 0), percent }].sort(
                          (a, b) => a.at - b.at
                        );
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
            <div
              className="mt-lane actions"
              style={{ height: ACTION_LANE }}
              onPointerDown={(event) => event.target === event.currentTarget && event.button === 0 && onSelection([])}
            >
              {timeline.actions.map((action, index) => {
                const shown = props.describeAction(action);
                const item: Item = { kind: "action", index };
                return (
                  <button
                    key={index}
                    type="button"
                    className={contains(selection, item) ? "mt-action selected" : "mt-action"}
                    data-index={index}
                    style={{ left: x(action.at), "--tint": rgb(shown.colour) } as CSSProperties}
                    title={shown.title}
                    // From the keyboard only: a pointer is handled when it goes down
                    onClick={(event) => event.detail === 0 && onSelection([item])}
                    onPointerDown={(event) => grab(event, item)}
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
          room={room}
          actionChoices={props.actionChoices ?? []}
          onClose={() => setMenu(null)}
          onAddSound={() => props.onAddSound?.(menu.at, menu.track)}
          onAddAction={(step) => {
            const actions = [...props.timeline.actions, { ...step, at: menu.at } as TimedStep];
            props.onChange({ ...props.timeline, actions });
            onSelection([{ kind: "action", index: props.timeline.actions.length }]);
          }}
          onDuplicate={duplicate}
          onDelete={(items) => {
            props.onChange(removedAll(props.timeline, items));
            onSelection([]);
          }}
        />
      )}
    </div>
  );
}

interface MenuAt {
  /** Where it opens, in the time line's own pixels, and how large the time line is */
  x: number;
  y: number;
  width: number;
  height: number;
  /** Opens upwards from the point, with this many pixels of room */
  up: boolean;
  room: number;
  /** The time and the track clicked: null on the actions' track */
  at: number;
  track: number | null;
  /** What it acts on: what was clicked, or the selection it is part of; null on an empty spot */
  targets: Item[] | null;
}

/** What a right click offers: duplicate or delete what it is on, else add a sound or an action at that point */
function TimelineMenu(props: {
  menu: MenuAt;
  room: { clips: number; actions: number };
  actionChoices: ActionChoice[];
  onClose: () => void;
  onAddSound: () => void;
  onAddAction: (step: Step) => void;
  onDuplicate: (items: Item[]) => void;
  onDelete: (items: Item[]) => void;
}) {
  const texts = useTexts();
  const { menu, room, onClose } = props;
  const full = { clips: room.clips <= 0, actions: room.actions <= 0 };
  const [actions, setActions] = useState(false);
  const element = useDismiss<HTMLDivElement>(true, onClose);
  const first = useRef<HTMLButtonElement>(null);
  useEffect(() => first.current?.focus(), [actions]);
  const run = (action: () => void) => () => {
    onClose();
    action();
  };
  const targets = menu.targets;
  const clips = targets?.filter((item) => item.kind === "clip").length ?? 0;
  const actionCount = (targets?.length ?? 0) - clips;
  const tooManyClips = clips > room.clips;
  const tooMany = tooManyClips || actionCount > room.actions;
  return (
    <div
      className="menu mt-menu"
      role="menu"
      ref={element}
      // Towards the side with room, never past the time line's edge; .menu's own top and left give way
      style={{
        ...(menu.x > menu.width - MENU_WIDTH ? { left: "auto", right: menu.width - menu.x } : { left: menu.x }),
        ...(menu.up ? { top: "auto", bottom: menu.height - menu.y } : { top: menu.y }),
        maxHeight: Math.min(MENU_HEIGHT - 20, menu.room),
      }}
      onContextMenu={(event) => event.preventDefault()}
      // Its keys are its own: the time line under it would move or delete what is selected
      onKeyDown={(event) => {
        event.stopPropagation();
        if (event.key === "Escape") onClose();
      }}
    >
      {targets ? (
        <>
          {targets.length > 1 && <span className="muted heading">{texts.selectedCount(targets.length)}</span>}
          <button
            type="button"
            role="menuitem"
            ref={first}
            disabled={tooMany}
            title={tooMany ? (tooManyClips ? texts.tooManyClips : texts.tooManyActions) : undefined}
            onClick={run(() => props.onDuplicate(targets))}
          >
            {texts.duplicate}
          </button>
          <button type="button" role="menuitem" className="danger" onClick={run(() => props.onDelete(targets))}>
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
  /** The sounds it stops when it starts */
  cuts: string[];
  playing: boolean;
  onSelect: () => void;
  onGrab: (event: ReactPointerEvent) => void;
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
    props.warned ? "warned" : props.cuts.length ? "cuts" : "",
    length.known ? "" : "guessed",
  ];
  const warning = props.warned ? texts.overlapsItself : props.cuts.length ? texts.cutsOthers(props.cuts) : undefined;
  const seconds = length.known ? texts.secondsOf(length.millis / 1000) : "?";
  // A short clip gives its room to the name: the length goes to its tooltip
  const roomy = width >= 60 + name.length * 7;
  // A very short one, its picture too
  const tiny = width < 80;
  return (
    <div
      className={classes.join(" ")}
      style={{ left: x(clip.at), width, "--tint": rgb(SOUND_COLOUR) } as CSSProperties}
      role="button"
      tabIndex={0}
      data-index={props.index}
      aria-pressed={props.selected}
      aria-label={texts.clipLabel(name, texts.secondsOf(clip.at / 1000))}
      title={warning ?? (roomy ? undefined : `${name} · ${seconds}`)}
      onPointerDown={props.onGrab}
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
        {!tiny && <PadPicture picture={soundOf(library, clip.id)?.picture} />}
        <span className="name">{name}</span>
        {roomy && <span className="length">{seconds}</span>}
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
              onPointerDown={(event) => props.onLevel(event, i, level)}
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
      {props.bridge && <span className="cut-handle" title={texts.dragToCut} onPointerDown={props.onCut} />}
    </div>
  );
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
  /** The sounds it stops when it starts */
  cuts: string[];
  onChange: (clip: Clip) => void;
  onChangeSound: () => void;
  /** Its sound no longer stops the others, in Voicemod */
  onKeepOthers: () => void;
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
        <PadPicture picture={soundOf(library, clip.id)?.picture} />
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
        aria-label={texts.deleteItem}
        title={texts.deleteItem}
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
      {props.cuts.length > 0 && (
        <p className="mt-warning">
          {texts.cutsOthers(props.cuts)}
          {bridge && (
            <button type="button" className="link" onClick={props.onKeepOthers}>
              {texts.keepOthers}
            </button>
          )}
        </p>
      )}
      {/* How to use it is the guide's own text there: a warning stays */}
      <p className={!bridge || props.warned ? "hint" : "hint how-to"}>
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

/** Several clips and actions selected: what can be done to them all */
export function GroupInspector(props: { count: number; onDuplicate: () => void; onRemove: () => void; full: boolean }) {
  const texts = useTexts();
  return (
    <div className="mt-inspector">
      <div className="identity">
        <div>
          <p className="title">{texts.selectedCount(props.count)}</p>
          <p className="muted">{texts.groupHint}</p>
        </div>
      </div>
      <span className="spacer" />
      <button type="button" className="button secondary" disabled={props.full} onClick={props.onDuplicate}>
        {texts.duplicate}
      </button>
      <button
        type="button"
        className="icon-button"
        aria-label={texts.deleteItem}
        title={texts.deleteItem}
        onClick={props.onRemove}
      >
        <TrashIcon />
      </button>
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
