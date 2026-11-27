import { type KeyboardEvent, type ReactNode, useEffect, useRef, useState } from "react";
import type { Library, LibraryItem, Rgb } from "../customPages";
import { useTexts } from "../i18n";
import { ChevronLeft, ChevronRight } from "../icons";
import { cutsOthers, duplicatedAll, freeTrack, isFree, type Item, removedAll } from "../macroEdit";
import { clipLength, MAX_CLIPS, MAX_STEPS, type TimedStep, type Timeline } from "../macros";
import { type ActionChoice, ClipInspector, GroupInspector, MacroTimeline, overlapsItself } from "./MacroTimeline";

type SoundKey = "music" | "horn" | "applause" | "news";

/** The sample's sounds and their lengths in seconds: the guide works without Voicemod */
const LENGTHS: Record<SoundKey, number> = { music: 6, horn: 1.5, applause: 3, news: 2.5 };
const SOUNDS = Object.keys(LENGTHS) as SoundKey[];
const MAX_UNDO = 50;

interface Start {
  clips: { id: SoundKey; track: number; at: number; cut?: number; volume?: { at: number; percent: number }[] }[];
  actions: TimedStep[];
  selection?: Item[];
  cursor?: number;
}

/** Each slide's time line as it opens: the reader tries on it, and nothing is saved */
const SLIDES: Start[] = [
  {
    clips: [
      { id: "music", track: 0, at: 0 },
      { id: "horn", track: 1, at: 1000 },
      { id: "applause", track: 1, at: 3000 },
    ],
    actions: [{ type: "randomVoice", at: 0 }],
  },
  { clips: [{ id: "music", track: 0, at: 0 }], actions: [], cursor: 2000 },
  {
    clips: [
      { id: "music", track: 0, at: 0 },
      { id: "horn", track: 1, at: 2000 },
    ],
    actions: [],
    selection: [{ kind: "clip", index: 1 }],
  },
  {
    clips: [
      {
        id: "music",
        track: 0,
        at: 0,
        cut: 4500,
        volume: [
          { at: 0, percent: 0 },
          { at: 1500, percent: 100 },
          { at: 3500, percent: 100 },
          { at: 4500, percent: 20 },
        ],
      },
    ],
    actions: [],
    selection: [{ kind: "clip", index: 0 }],
  },
  {
    clips: [
      { id: "horn", track: 0, at: 500 },
      { id: "applause", track: 1, at: 1000 },
    ],
    actions: [{ type: "switch", switch: "muteMic", state: "on", at: 500 }],
    selection: [
      { kind: "clip", index: 0 },
      { kind: "clip", index: 1 },
    ],
  },
  {
    clips: [
      { id: "music", track: 0, at: 0 },
      { id: "horn", track: 1, at: 800 },
      { id: "horn", track: 1, at: 1800 },
      { id: "news", track: 1, at: 3500 },
    ],
    actions: [],
  },
];

interface Props {
  actionChoices: ActionChoice[];
  describeAction: (action: TimedStep) => { title: string; icon: ReactNode; colour: Rgb };
  onClose: () => void;
}

/** How the Macro section works, slide by slide, each with a small time line to try */
export function MacroGuide(props: Props) {
  const texts = useTexts();
  const words = texts.macroGuide;
  const [slide, setSlide] = useState(0);
  const dialog = useRef<HTMLDialogElement>(null);
  const next = useRef<HTMLButtonElement>(null);
  const [opener] = useState(() => document.activeElement as HTMLElement | null);
  const last = slide === SLIDES.length - 1;

  // Escape in a menu, or with something selected on the time line, lets go of that alone
  const escapeKept = useRef(false);

  // A control that went with the last slide, as Back with the first one, leaves its focus to Next: the arrows keep working
  useEffect(() => {
    if (!dialog.current?.contains(document.activeElement)) next.current?.focus();
  }, [slide]);

  useEffect(() => {
    dialog.current?.showModal();
    next.current?.focus();
    return () => {
      if (opener?.isConnected) opener.focus();
    };
  }, [opener]);

  const go = (to: number) => setSlide(Math.max(0, Math.min(SLIDES.length - 1, to)));
  const onKeyDown = (event: KeyboardEvent<HTMLDialogElement>) => {
    // The time line, its menus and the fields keep their arrows
    if (event.target instanceof Element && event.target.closest(".mt, .mt-inspector, .menu")) return;
    if (event.key === "ArrowRight") go(slide + 1);
    else if (event.key === "ArrowLeft") go(slide - 1);
    else return;
    event.preventDefault();
  };

  return (
    <dialog
      ref={dialog}
      className="tour macro-guide"
      aria-label={words.label}
      onKeyDown={onKeyDown}
      onKeyDownCapture={(event) => {
        const target = event.target instanceof Element ? event.target : null;
        const selected = target?.closest(".mt") && dialog.current?.querySelector(".mt .selected");
        escapeKept.current = event.key === "Escape" && (!!target?.closest(".menu, .search-menu") || !!selected);
      }}
      onCancel={(event) => {
        event.preventDefault();
        if (!escapeKept.current) props.onClose();
      }}
    >
      <header>
        <div className="tour-dots">
          {SLIDES.map((_, i) => (
            <button
              key={i}
              type="button"
              className={i < slide ? "seen" : undefined}
              aria-label={texts.tour.goTo(i + 1)}
              aria-current={i === slide ? "step" : undefined}
              onClick={() => go(i)}
            >
              <span />
            </button>
          ))}
        </div>
        <button type="button" className="tour-skip" onClick={props.onClose}>
          {words.close}
        </button>
      </header>

      <div className="guide-body">
        <div className="tour-text guide-text" aria-live="polite">
          <p className="tour-counter">{texts.tour.counter(slide + 1, SLIDES.length)}</p>
          <h2>{words.slides[slide].title}</h2>
          <ul>
            {words.slides[slide].points.map((point) => (
              <li key={point}>{point}</li>
            ))}
          </ul>
          <p className="guide-try">{words.slides[slide].tryIt}</p>
        </div>

        <div className="guide-stage" data-menu-frame>
          <GuideTimeline key={slide} start={SLIDES[slide]} {...props} />
        </div>
      </div>

      <footer>
        {slide > 0 && (
          <button type="button" className="button ghost" onClick={() => go(slide - 1)}>
            <ChevronLeft />
            {texts.tour.back}
          </button>
        )}
        <button ref={next} type="button" className="button" onClick={() => (last ? props.onClose() : go(slide + 1))}>
          {last ? words.done : texts.tour.next}
          <ChevronRight />
        </button>
      </footer>
    </dialog>
  );
}

/** One slide's time line, with the box of what is selected, as in the editor */
function GuideTimeline(props: Props & { start: Start }) {
  const texts = useTexts();
  const words = texts.macroGuide;
  const { start } = props;
  const [library, setLibrary] = useState<Library>(() => ({
    soundboards: [{ id: "guide", name: "", custom: false, sounds: SOUNDS.map((id) => sampleSound(id, words.sounds[id])) }],
    voices: [],
    favourites: false,
  }));
  const [timeline, setTimeline] = useState<Timeline>(() => ({
    clips: start.clips.map((clip) => ({ ...clip, name: words.sounds[clip.id] })),
    actions: start.actions,
  }));
  const [selection, setSelection] = useState<Item[]>(start.selection ?? []);
  const [cursor, setCursor] = useState(start.cursor ?? 0);
  const [past, setPast] = useState<Timeline[]>([]);
  const [future, setFuture] = useState<Timeline[]>([]);

  const change = (next: Timeline) => {
    if (JSON.stringify(next) === JSON.stringify(timeline)) return;
    setPast([...past, timeline].slice(-MAX_UNDO));
    setFuture([]);
    setTimeline(next);
  };
  const undo = () => {
    const previous = past[past.length - 1];
    if (!previous) return;
    setPast(past.slice(0, -1));
    setFuture([timeline, ...future]);
    setTimeline(previous);
    setSelection([]);
  };
  const redo = () => {
    const following = future[0];
    if (!following) return;
    setFuture(future.slice(1));
    setPast([...past, timeline]);
    setTimeline(following);
    setSelection([]);
  };
  // From anywhere while the guide is open, as a removed control leaves the focus to the page
  const undoKeys = useRef({ undo, redo });
  undoKeys.current = { undo, redo };
  useEffect(() => {
    const keys = (event: globalThis.KeyboardEvent) => {
      const key = event.key.toLowerCase();
      if (!(event.ctrlKey || event.metaKey) || (key !== "z" && key !== "y")) return;
      if ((event.target as HTMLElement | null)?.closest("input, textarea")) return;
      event.preventDefault();
      if (key === "y" || event.shiftKey) undoKeys.current.redo();
      else undoKeys.current.undo();
    };
    window.addEventListener("keydown", keys);
    return () => window.removeEventListener("keydown", keys);
  }, []);

  const only = selection.length === 1 ? selection[0] : null;
  const selectedClip = only?.kind === "clip" ? timeline.clips[only.index] : undefined;
  return (
    <div className="guide-timeline">
      <MacroTimeline
        timeline={timeline}
        library={library}
        running={null}
        bridge
        selection={selection}
        onSelection={setSelection}
        onChange={change}
        cursor={cursor}
        onCursor={setCursor}
        describeAction={props.describeAction}
        actionChoices={props.actionChoices}
        onAddSound={(at, track) => {
          // On the track clicked if it is free there, as in the editor
          const clip = { id: "horn", name: words.sounds.horn, track: 0, at };
          const millis = clipLength(clip, library).millis;
          clip.track =
            track !== null && isFree(timeline, track, at, millis, library) ? track : freeTrack(timeline, at, millis, library);
          change({ ...timeline, clips: [...timeline.clips, clip] });
          setSelection([{ kind: "clip", index: timeline.clips.length }]);
        }}
        history={{ canUndo: past.length > 0, canRedo: future.length > 0, undo, redo }}
      />
      {selectedClip && only && (
        <ClipInspector
          clip={selectedClip}
          library={library}
          bridge
          warned={overlapsItself(timeline, only.index, library)}
          cuts={cutsOthers(timeline, only.index, library)}
          onChange={(clip) =>
            change({ ...timeline, clips: timeline.clips.map((other, i) => (i === only.index ? clip : other)) })
          }
          onChangeSound={() => {
            // The next of the sample's sounds: Voicemod's own are not chosen here
            const id = SOUNDS[(SOUNDS.indexOf(selectedClip.id as SoundKey) + 1) % SOUNDS.length];
            const clip = { ...selectedClip, id, name: words.sounds[id], cut: null, volume: [] };
            change({ ...timeline, clips: timeline.clips.map((other, i) => (i === only.index ? clip : other)) });
          }}
          onKeepOthers={() =>
            setLibrary({
              ...library,
              soundboards: library.soundboards.map((board) => ({
                ...board,
                sounds: board.sounds.map((sound) => (sound.id === selectedClip.id ? { ...sound, stopsOthers: false } : sound)),
              })),
            })
          }
          onRemove={() => {
            change(removedAll(timeline, selection));
            setSelection([]);
          }}
        />
      )}
      {selection.length > 1 && (
        <GroupInspector
          count={selection.length}
          full={
            timeline.clips.length + selection.filter((item) => item.kind === "clip").length > MAX_CLIPS ||
            timeline.actions.length + selection.filter((item) => item.kind === "action").length > MAX_STEPS
          }
          onDuplicate={() => {
            const copied = duplicatedAll(timeline, selection, library);
            change(copied.timeline);
            setSelection(copied.items);
          }}
          onRemove={() => {
            change(removedAll(timeline, selection));
            setSelection([]);
          }}
        />
      )}
    </div>
  );
}

function sampleSound(id: SoundKey, name: string): LibraryItem {
  return {
    id,
    name,
    enabled: true,
    picture: null,
    length: LENGTHS[id],
    mode: "PlayRestart",
    // As some of Voicemod's sounds, the announcement stops the others when it starts
    stopsOthers: id === "news",
  };
}
