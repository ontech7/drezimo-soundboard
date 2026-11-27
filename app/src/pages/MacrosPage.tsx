import { invoke } from "@tauri-apps/api/core";
import { type CSSProperties, type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { Segmented, Toggle } from "../controls";
import {
  ACTION_COLOURS,
  type CustomPage,
  FAVOURITES_ID,
  type Library,
  type LibraryItem,
  MACRO_COLOUR,
  type Rgb,
  rgb,
  shortened,
  SOUND_COLOUR,
  useCustomPages,
  useLibrary,
  VOICE_COLOUR,
  VOICES_ID,
} from "../customPages";
import { useDismiss } from "../dismiss";
import { ColourPicker, RenameField } from "../editing";
import { type Texts, useTexts } from "../i18n";
import {
  BackgroundIcon,
  ClockIcon,
  DotsIcon,
  GoToIcon,
  HearMeIcon,
  MacroIcon,
  MicMuteIcon,
  MuteMeIcon,
  PlayIcon,
  PlusIcon,
  SearchIcon,
  ShuffleIcon,
  SoundIcon,
  StopIcon,
  TrashIcon,
  VoiceChangerIcon,
  VoiceIcon,
} from "../icons";
import {
  clipLength,
  type Macro,
  MAX_CLIPS,
  MAX_MACROS,
  MAX_STEPS,
  MAX_WAIT,
  MIN_WAIT,
  newMacroId,
  readSoundLengths,
  type Step,
  type Switch,
  SWITCHES,
  stopMacro,
  type Timeline as Tracks,
  timelineOf,
  tryMacro,
  useMacros,
} from "../macros";
import { cutsOthers, duplicatedAll, freeTrack, isFree, type Item, removedAll } from "../macroEdit";
import {
  type ActionChoice,
  AddToTimeline,
  ClipInspector,
  GroupInspector,
  MacroTimeline,
  overlapsItself,
} from "./MacroTimeline";
import { PadPicture } from "../PadPicture";
import { SearchSelect } from "../SearchSelect";
import type { Status } from "../status";
import { Dialog } from "./PadPicker";

const MAX_NAME = 24;
/** The changes a macro can go back through */
const MAX_UNDO = 50;

interface History {
  /** The macro they belong to */
  id: string;
  past: Tracks[];
  future: Tracks[];
}
const WAIT_COLOUR: Rgb = [161, 161, 170];

const SWITCH_ICONS: Record<Switch, ReactNode> = {
  muteMic: <MicMuteIcon />,
  voiceChanger: <VoiceChangerIcon />,
  hearMyself: <HearMeIcon />,
  muteForMe: <MuteMeIcon />,
  background: <BackgroundIcon />,
};

interface Target {
  id: string;
  name: string;
}

/** The pages a step can open, as the board has them */
function targetsOf(pages: CustomPage[], library: Library | null, texts: Texts): Target[] {
  return [
    ...pages.map((page) => ({ id: page.id, name: page.name })),
    ...(library?.favourites ? [{ id: FAVOURITES_ID, name: texts.favouritesPage }] : []),
    ...(library?.voices.some((voice) => voice.enabled) ? [{ id: VOICES_ID, name: texts.voicesPage }] : []),
    ...(library?.soundboards ?? []).map((soundboard) => ({ id: soundboard.id, name: soundboard.name })),
  ];
}

function findSound(library: Library | null, id: string): LibraryItem | undefined {
  for (const soundboard of library?.soundboards ?? []) {
    const sound = soundboard.sounds.find((item) => item.id === id);
    if (sound) return sound;
  }
  return undefined;
}

interface Described {
  title: string;
  detail: string;
  icon: ReactNode;
  colour: Rgb;
}

export function describe(step: Step, texts: Texts, library: Library | null, targets: Target[]): Described {
  switch (step.type) {
    case "sound": {
      const sound = findSound(library, step.id);
      const length = sound?.length;
      let detail = texts.nextAtOnce;
      if (!sound && library?.soundboards.length) detail = texts.soundMissing;
      else if (step.waitEnd) detail = texts.waitsForEnd(length == null ? texts.unknownLength : texts.secondsOf(length));
      return { title: texts.playSound(sound?.name ?? step.name), detail, icon: <SoundIcon />, colour: SOUND_COLOUR };
    }
    case "wait":
      return {
        title: texts.waitFor(texts.secondsOf(step.millis / 1000)),
        detail: "",
        icon: <ClockIcon />,
        colour: WAIT_COLOUR,
      };
    case "voice": {
      const name = library?.voices.find((voice) => voice.id === step.id)?.name ?? step.name;
      return { title: texts.voiceIs(name), detail: texts.voicemodVoice, icon: <VoiceIcon />, colour: VOICE_COLOUR };
    }
    case "randomVoice":
      return { title: texts.randomVoiceStep, detail: texts.randomVoiceText, icon: <ShuffleIcon />, colour: VOICE_COLOUR };
    case "previousVoice":
      return { title: texts.previousVoiceStep, detail: texts.previousVoiceText, icon: <VoiceIcon />, colour: VOICE_COLOUR };
    case "switch": {
      const states = step.switch === "muteMic" ? texts.micStates : texts.switchStates;
      return {
        title: `${texts.switchNames[step.switch]}: ${states[step.state].toLowerCase()}`,
        detail: step.state === "toggle" ? texts.toggleText : "",
        icon: SWITCH_ICONS[step.switch],
        colour: ACTION_COLOURS[step.switch],
      };
    }
    case "stopSounds":
      return { title: texts.stopStep, detail: texts.stopText, icon: <StopIcon />, colour: ACTION_COLOURS.stopSounds };
    case "goToPage": {
      const target = targets.find((page) => page.id === step.page);
      return {
        title: target ? texts.goTo(target.name) : texts.actionNames.goToPage,
        detail: texts.goToText,
        icon: <GoToIcon />,
        colour: ACTION_COLOURS.goToPage,
      };
    }
  }
}

export function Tinted({ colour, children }: { colour: Rgb; children: ReactNode }) {
  return (
    <span className="picture action" style={{ "--tint": rgb(colour) } as CSSProperties}>
      {children}
    </span>
  );
}

export function MacrosPage({ status }: { status: Status }) {
  const texts = useTexts();
  const [macros, save, saveError, flush] = useMacros();
  const [pages] = useCustomPages();
  const library = useLibrary();
  const [current, setCurrent] = useState(0);
  // One clip or action, or several with Shift+click
  const [selection, setSelection] = useState<Item[]>([]);
  // The macro's time lines before and after its changes, for undo and redo, until another macro is shown
  const [history, setHistory] = useState<History>({ id: "", past: [], future: [] });
  // Where a sound or an action added next goes, in milliseconds
  const [cursor, setCursor] = useState(0);
  // The clip whose sound is being chosen, or "new" for a sound to add
  const [choosing, setChoosing] = useState<number | "new" | null>(null);
  // The track a sound added from the right-click menu goes on, if free there
  const [newTrack, setNewTrack] = useState<number | null>(null);
  const targets = useMemo(() => targetsOf(pages ?? [], library, texts), [pages, library, texts]);
  // Undo and redo from anywhere on the page, but in a text field, which has its own
  const undoKeys = useRef<{ undo: () => void; redo: () => void } | null>(null);
  useEffect(() => {
    const keys = (event: KeyboardEvent) => {
      const key = event.key.toLowerCase();
      if (!(event.ctrlKey || event.metaKey) || (key !== "z" && key !== "y")) return;
      if ((event.target as HTMLElement | null)?.closest("input, textarea, [contenteditable]")) return;
      event.preventDefault();
      if (key === "y" || event.shiftKey) undoKeys.current?.redo();
      else undoKeys.current?.undo();
    };
    window.addEventListener("keydown", keys);
    return () => window.removeEventListener("keydown", keys);
  }, []);

  // The time line draws each sound as long as it is
  useEffect(() => readSoundLengths(), []);

  if (!macros) return null;

  const blank = (): Macro => ({
    id: newMacroId(),
    name: texts.macroNumber(macros.length + 1),
    colour: null,
    again: "stop",
    repeat: 1,
    restore: true,
    steps: [],
  });
  const add = (item: Macro) => {
    save([...macros, item]);
    setCurrent(macros.length);
    setSelection([]);
    setCursor(0);
  };

  if (macros.length === 0) {
    return (
      <section className="card empty-pages">
        <Tinted colour={MACRO_COLOUR}>
          <MacroIcon />
        </Tinted>
        <h2>{texts.macrosEmptyTitle}</h2>
        <p className="muted">{texts.macrosEmptyText}</p>
        <div className="actions">
          <button type="button" className="button" onClick={() => add(blank())}>
            {texts.newMacro}
          </button>
        </div>
      </section>
    );
  }

  const index = Math.min(current, macros.length - 1);
  const item = macros[index];
  const saveMacro = (next: Macro) => save(macros.map((other, i) => (i === index ? next : other)));
  const timeline = timelineOf(item, library);
  const kept = history.id === item.id ? history : { id: item.id, past: [], future: [] };
  // Once edited, a macro from before the time line keeps only its time line
  const store = (next: Tracks) => saveMacro({ ...item, steps: [], timeline: next });
  const setTimeline = (next: Tracks) => {
    // A change that changes nothing, as an arrow at the start, is no step to undo
    if (JSON.stringify(next) === JSON.stringify(timeline)) return;
    setHistory({ id: item.id, past: [...kept.past, timeline].slice(-MAX_UNDO), future: [] });
    store(next);
  };
  const undoable = {
    canUndo: kept.past.length > 0,
    canRedo: kept.future.length > 0,
    undo: () => {
      const previous = kept.past[kept.past.length - 1];
      if (!previous) return;
      setHistory({ id: item.id, past: kept.past.slice(0, -1), future: [timeline, ...kept.future] });
      store(previous);
      // The indices may now point at other items
      setSelection([]);
    },
    redo: () => {
      const following = kept.future[0];
      if (!following) return;
      setHistory({ id: item.id, past: [...kept.past, timeline], future: kept.future.slice(1) });
      store(following);
      setSelection([]);
    },
  };
  undoKeys.current = undoable;
  const running = status.runningMacro?.id === item.id ? status.runningMacro : null;
  const playable = status.voicemod.state === "connected" && timeline.clips.length + timeline.actions.length > 0;
  const only = selection.length === 1 ? selection[0] : null;
  const selectedClip = only?.kind === "clip" ? timeline.clips[only.index] : undefined;
  const selectedAction = only?.kind === "action" ? timeline.actions[only.index] : undefined;

  const places = (pages ?? []).flatMap((page) =>
    page.pads.flatMap((pad, i) => (pad?.kind === "macro" && pad.id === item.id ? [`${page.name} ${i + 1}`] : []))
  );

  const pickSound = (sound: LibraryItem) => {
    if (choosing === "new") {
      const clip = { id: sound.id, name: sound.name, track: 0, at: cursor };
      const millis = clipLength(clip, library).millis;
      const asked = newTrack !== null && isFree(timeline, newTrack, cursor, millis, library);
      clip.track = asked && newTrack !== null ? newTrack : freeTrack(timeline, cursor, millis, library);
      setTimeline({ ...timeline, clips: [...timeline.clips, clip] });
      setSelection([{ kind: "clip", index: timeline.clips.length }]);
      // The next one after it, as a sequence is most often made
      setCursor(cursor + millis);
    } else if (choosing !== null) {
      const clips = timeline.clips.map((clip, i) =>
        i === choosing ? { ...clip, id: sound.id, name: sound.name, cut: null, volume: [] } : clip
      );
      setTimeline({ ...timeline, clips });
    }
    setChoosing(null);
    setNewTrack(null);
  };

  return (
    <>
      {saveError && <p className="note">{texts.macrosSaveFailed(saveError)}</p>}
      <section className="card macro-editor" aria-label={item.name}>
        <MacroTabs
          macros={macros}
          index={index}
          onSelect={(i) => {
            setCurrent(i);
            setSelection([]);
            setCursor(0);
          }}
          onAdd={() => add(blank())}
          onRename={(name) => saveMacro({ ...item, name })}
          onDuplicate={() => {
            const copy = { ...item, id: newMacroId(), name: shortened(texts.copyOf(item.name), MAX_NAME) };
            save([...macros.slice(0, index + 1), copy, ...macros.slice(index + 1)]);
            setCurrent(index + 1);
          }}
          onDelete={() => {
            save(macros.filter((_, i) => i !== index));
            setCurrent(Math.max(0, index - 1));
            setSelection([]);
          }}
        />

        <div className="macro-head">
          <ColourPicker
            colour={item.colour}
            shown={item.colour ?? MACRO_COLOUR}
            below
            onChange={(colour) => saveMacro({ ...item, colour })}
          />
          <p className="muted used">{texts.usedOn(places.length, places.join(", "))}</p>
          <button
            type="button"
            className={running ? "button" : "button outline"}
            disabled={!running && !playable}
            // The latest change saved first, so Try runs what is shown
            onClick={() => (running ? stopMacro() : flush().then(() => tryMacro(item.id)))}
          >
            {running ? <StopIcon /> : <PlayIcon />}
            {running ? texts.stopMacro : texts.tryMacro}
          </button>
        </div>

        <MacroTimeline
          key={item.id}
          timeline={timeline}
          library={library}
          running={running}
          bridge={status.singleSoundStop}
          selection={selection}
          onSelection={setSelection}
          onChange={setTimeline}
          history={undoable}
          cursor={cursor}
          onCursor={setCursor}
          describeAction={(action) => describe(action, texts, library, targets)}
          actionChoices={actionChoices(texts, targets[0]?.id ?? VOICES_ID)}
          onAddSound={(at, track) => {
            setCursor(at);
            setNewTrack(track);
            setChoosing("new");
          }}
        />
        {timeline.clips.length + timeline.actions.length === 0 && <p className="muted no-steps">{texts.noClips}</p>}

        {selectedClip && only && (
          <ClipInspector
            clip={selectedClip}
            library={library}
            bridge={status.singleSoundStop}
            warned={overlapsItself(timeline, only.index, library)}
            cuts={cutsOthers(timeline, only.index, library)}
            onChange={(clip) =>
              setTimeline({ ...timeline, clips: timeline.clips.map((other, i) => (i === only.index ? clip : other)) })
            }
            onChangeSound={() => setChoosing(only.index)}
            onKeepOthers={() =>
              invoke("set_sound_stops_others", { id: selectedClip.id, stopsOthers: false }).catch((error) =>
                console.error("cannot change the sound in Voicemod", error)
              )
            }
            onRemove={() => {
              setTimeline(removedAll(timeline, selection));
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
              setTimeline(copied.timeline);
              setSelection(copied.items);
            }}
            onRemove={() => {
              setTimeline(removedAll(timeline, selection));
              setSelection([]);
            }}
          />
        )}
        {selectedAction && only && (
          <div className="mt-inspector">
            <div className="identity">
              <Tinted colour={describe(selectedAction, texts, library, targets).colour}>
                {describe(selectedAction, texts, library, targets).icon}
              </Tinted>
              <div>
                <p className="title">{describe(selectedAction, texts, library, targets).title}</p>
                <p className="muted">{texts.actionAt(texts.secondsOf(selectedAction.at / 1000))}</p>
              </div>
            </div>
            <StepEditor
              step={selectedAction}
              library={library}
              targets={targets}
              onChange={(step) =>
                setTimeline({
                  ...timeline,
                  actions: timeline.actions.map((other, i) =>
                    i === only.index ? ({ ...step, at: other.at } as typeof other) : other
                  ),
                })
              }
              onChooseSound={() => {}}
            />
            <span className="spacer" />
            <button
              type="button"
              className="icon-button"
              aria-label={texts.deleteItem}
              title={texts.deleteItem}
              onClick={() => {
                setTimeline(removedAll(timeline, selection));
                setSelection([]);
              }}
            >
              <TrashIcon />
            </button>
          </div>
        )}

        <AddToTimeline
          full={{ clips: timeline.clips.length >= MAX_CLIPS, actions: timeline.actions.length >= MAX_STEPS }}
          onSound={() => {
            setNewTrack(null);
            setChoosing("new");
          }}
          actionMenu={
            <AddStep
              actionsOnly
              disabled={timeline.actions.length >= MAX_STEPS}
              firstTarget={targets[0]?.id ?? VOICES_ID}
              onAdd={(step) => {
                if (!step || step.type === "sound" || step.type === "wait") return;
                setTimeline({ ...timeline, actions: [...timeline.actions, { ...step, at: cursor }] });
                setSelection([{ kind: "action", index: timeline.actions.length }]);
              }}
            />
          }
        />
        <p className="hint mt-hint">{texts.timelineHint}</p>

        <div className="macro-options">
          <div className="option">
            <span className="muted">{texts.pressedAgain}</span>
            <Segmented
              label={texts.pressedAgain}
              options={[
                { value: "stop", label: texts.againStop },
                { value: "restart", label: texts.againRestart },
              ]}
              value={item.again}
              onChange={(again) => saveMacro({ ...item, again })}
            />
          </div>
          <div className="option">
            <span className="muted">{texts.repeatLabel}</span>
            <Segmented
              label={texts.repeatLabel}
              options={[
                { value: 1, label: texts.repeatOnce },
                { value: 2, label: "2" },
                { value: 3, label: "3" },
                { value: 0, label: texts.repeatAlways },
              ]}
              value={item.repeat}
              onChange={(repeat) => saveMacro({ ...item, repeat })}
            />
          </div>
          <div className="option" title={texts.restoreText}>
            <Toggle label={texts.restoreLabel} checked={item.restore} onChange={(restore) => saveMacro({ ...item, restore })} />
            <span className="muted">{texts.restoreLabel}</span>
          </div>
        </div>
      </section>

      {choosing !== null && <SoundChooser library={library} onPick={pickSound} onClose={() => setChoosing(null)} />}
    </>
  );
}

interface TabsProps {
  macros: Macro[];
  index: number;
  onSelect: (index: number) => void;
  onAdd: () => void;
  onRename: (name: string) => void;
  onDuplicate: () => void;
  onDelete: () => void;
}

function MacroTabs(props: TabsProps) {
  const texts = useTexts();
  const [renaming, setRenaming] = useState(false);
  const [menu, setMenu] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const options = useDismiss<HTMLDivElement>(menu, () => {
    setMenu(false);
    setConfirming(false);
  });
  const run = (action: () => void) => () => {
    setMenu(false);
    setConfirming(false);
    action();
  };
  const full = props.macros.length >= MAX_MACROS;

  return (
    <div className="page-tabs">
      <div className="tabs" role="group" aria-label={texts.yourMacros}>
        {props.macros.map((item, i) =>
          renaming && i === props.index ? (
            <RenameField
              key={item.id}
              name={item.name}
              label={texts.macroName}
              max={MAX_NAME}
              onDone={(name) => {
                setRenaming(false);
                if (name) props.onRename(name);
              }}
            />
          ) : (
            <button
              key={item.id}
              type="button"
              className={i === props.index ? "chip selected" : "chip"}
              aria-pressed={i === props.index}
              onClick={() => props.onSelect(i)}
              onDoubleClick={() => i === props.index && setRenaming(true)}
            >
              <span className="dot" style={{ "--led": rgb(item.colour ?? MACRO_COLOUR) } as CSSProperties} />
              {item.name}
            </button>
          )
        )}
      </div>
      <button
        type="button"
        className="chip add"
        aria-label={texts.newMacro}
        title={full ? texts.tooManyMacros : texts.newMacro}
        disabled={full}
        onClick={props.onAdd}
      >
        <PlusIcon />
      </button>
      <span className="spacer" />
      <div className="anchor" ref={options}>
        <button
          type="button"
          className="icon-button small"
          aria-label={texts.macroOptions}
          title={texts.macroOptions}
          aria-expanded={menu}
          onClick={() => setMenu(!menu)}
        >
          <DotsIcon />
        </button>
        {menu && (
          <div className="menu right">
            <button type="button" onClick={run(() => setRenaming(true))}>
              {texts.rename}
            </button>
            <button type="button" disabled={full} onClick={run(props.onDuplicate)}>
              {texts.duplicate}
            </button>
            {/* Deleting takes a second click: its pads empty */}
            <button type="button" className="danger" onClick={confirming ? run(props.onDelete) : () => setConfirming(true)}>
              {confirming ? `${texts.deletePage} “${props.macros[props.index].name}”?` : texts.deletePage}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

function StepEditor(props: {
  step: Step;
  library: Library | null;
  targets: Target[];
  onChange: (step: Step) => void;
  onChooseSound: () => void;
}) {
  const texts = useTexts();
  const { step, library, onChange } = props;
  const licensed = library?.voices.filter((voice) => voice.enabled) ?? [];

  switch (step.type) {
    case "sound": {
      const sound = findSound(library, step.id);
      return (
        <div className="step-editor">
          <div className="sound-line">
            <span className="sound-chip">
              <PadPicture picture={sound?.picture} />
              <span className="name">{sound?.name ?? step.name}</span>
              {sound?.length != null && <span className="muted">{texts.secondsOf(sound.length)}</span>}
            </span>
            <button type="button" className="button secondary" onClick={props.onChooseSound}>
              {texts.changeSound}
            </button>
          </div>
          <div className="option">
            <Toggle label={texts.waitEnd} checked={step.waitEnd} onChange={(waitEnd) => onChange({ ...step, waitEnd })} />
            <span>
              {texts.waitEnd}
              <span className="muted block">{step.waitEnd ? texts.waitEndOn : texts.waitEndOff}</span>
            </span>
          </div>
        </div>
      );
    }
    case "wait": {
      const seconds = step.millis / 1000;
      const set = (value: number) =>
        onChange({ ...step, millis: Math.round(Math.min(MAX_WAIT, Math.max(MIN_WAIT, value * 1000)) / 100) * 100 });
      return (
        <div className="step-editor">
          <label className="option">
            <span className="muted">{texts.howLong}</span>
            <input
              type="range"
              min={0.1}
              max={10}
              step={0.1}
              value={Math.min(seconds, 10)}
              style={{ "--fill": `${((Math.min(seconds, 10) - 0.1) / 9.9) * 100}%` } as CSSProperties}
              onChange={(event) => set(Number(event.target.value))}
            />
            <SecondsField key={step.millis} label={texts.howLong} seconds={seconds} onDone={set} />
            <span className="muted">s</span>
          </label>
        </div>
      );
    }
    case "voice":
    case "randomVoice":
    case "previousVoice": {
      const mode = step.type === "voice" ? "one" : step.type === "randomVoice" ? "random" : "previous";
      const firstVoice = licensed[0];
      return (
        <div className="step-editor">
          <Segmented
            label={texts.whichVoice}
            options={[
              { value: "one", label: texts.voiceModes.one },
              { value: "random", label: texts.voiceModes.random },
              { value: "previous", label: texts.voiceModes.previous },
            ]}
            value={mode}
            onChange={(next) => {
              if (next === "random") onChange({ type: "randomVoice" });
              else if (next === "previous") onChange({ type: "previousVoice" });
              else if (firstVoice) onChange({ type: "voice", id: firstVoice.id, name: firstVoice.name });
            }}
          />
          {step.type === "voice" && (
            <div className="option">
              <span className="muted">{texts.whichVoice}</span>
              <SearchSelect
                label={texts.whichVoice}
                options={licensed.map((voice) => ({ value: voice.id, label: voice.name }))}
                value={step.id}
                placeholder={step.name}
                onChange={(id) => {
                  const voice = licensed.find((other) => other.id === id);
                  if (voice) onChange({ type: "voice", id: voice.id, name: voice.name });
                }}
                searchLabel={texts.searchVoice}
                empty={texts.noVoice}
              />
            </div>
          )}
        </div>
      );
    }
    case "switch": {
      const states = step.switch === "muteMic" ? texts.micStates : texts.switchStates;
      return (
        <div className="step-editor">
          <Segmented
            label={texts.switchNames[step.switch]}
            options={(["on", "off", "toggle"] as const).map((value) => ({ value, label: states[value] }))}
            value={step.state}
            onChange={(state) => onChange({ ...step, state })}
          />
        </div>
      );
    }
    case "stopSounds":
      return (
        <div className="step-editor">
          <p className="muted">{texts.stopText}</p>
        </div>
      );
    case "goToPage":
      return (
        <div className="step-editor">
          <div className="option">
            <span className="muted">{texts.pageToOpen}</span>
            <SearchSelect
              label={texts.pageToOpen}
              options={props.targets.map((target) => ({ value: target.id, label: target.name }))}
              value={step.page}
              placeholder={texts.choosePage}
              onChange={(page) => onChange({ ...step, page })}
              searchLabel={texts.searchPage}
              empty={texts.noPage}
            />
          </div>
        </div>
      );
  }
}

/** Typed freely, kept within the limits when left or on Enter */
function SecondsField(props: { label: string; seconds: number; onDone: (seconds: number) => void }) {
  const [draft, setDraft] = useState(String(props.seconds));
  const done = () => {
    const value = Number(draft.replace(",", "."));
    if (Number.isFinite(value) && value > 0) props.onDone(value);
    else setDraft(String(props.seconds));
  };
  return (
    <input
      type="text"
      inputMode="decimal"
      className="seconds"
      aria-label={props.label}
      value={draft}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={done}
      onKeyDown={(event) => {
        if (event.key === "Enter") done();
      }}
    />
  );
}

type Kind = keyof Texts["stepKinds"];

const STEP_KINDS: { kind: Kind; icon: ReactNode; colour: Rgb }[] = [
  { kind: "sound", icon: <SoundIcon />, colour: SOUND_COLOUR },
  { kind: "wait", icon: <ClockIcon />, colour: WAIT_COLOUR },
  { kind: "voice", icon: <VoiceIcon />, colour: VOICE_COLOUR },
  ...SWITCHES.map((kind) => ({ kind, icon: SWITCH_ICONS[kind], colour: ACTION_COLOURS[kind] })),
  { kind: "stopSounds", icon: <StopIcon />, colour: ACTION_COLOURS.stopSounds },
  { kind: "goToPage", icon: <GoToIcon />, colour: ACTION_COLOURS.goToPage },
];

/** A new step of that kind; a sound is chosen first */
function stepOf(kind: Kind, firstTarget: string): Step | null {
  if (kind === "sound") return null;
  if (kind === "wait") return { type: "wait", millis: 1000 };
  if (kind === "voice") return { type: "randomVoice" };
  if (kind === "stopSounds") return { type: "stopSounds" };
  if (kind === "goToPage") return { type: "goToPage", page: firstTarget };
  return { type: "switch", switch: kind, state: "on" };
}

/** The actions the time line's menus add */
function actionChoices(texts: Texts, firstTarget: string): ActionChoice[] {
  return STEP_KINDS.flatMap(({ kind, icon, colour }) => {
    const step = stepOf(kind, firstTarget);
    return step && step.type !== "wait" ? [{ key: kind, label: texts.stepKinds[kind][0], icon, colour, step }] : [];
  });
}

/** The steps to add; a sound is chosen first. On the time line, only the actions: sounds have their own button. */
function AddStep(props: { disabled: boolean; firstTarget: string; actionsOnly?: boolean; onAdd: (step: Step | null) => void }) {
  const texts = useTexts();
  const [open, setOpen] = useState(false);
  const menu = useDismiss<HTMLDivElement>(open, () => setOpen(false));
  const make = (kind: Kind) => stepOf(kind, props.firstTarget);
  const kinds = props.actionsOnly ? STEP_KINDS.filter(({ kind }) => kind !== "sound" && kind !== "wait") : STEP_KINDS;
  return (
    <div className="anchor add-step" ref={menu}>
      <button
        type="button"
        className="add-step-button"
        aria-expanded={open}
        disabled={props.disabled}
        onClick={() => setOpen(!open)}
      >
        <PlusIcon />
        {props.actionsOnly ? texts.addAction : texts.addStep}
      </button>
      {open && (
        <div className="step-kinds">
          {kinds.map(({ kind, icon, colour }) => (
            <button
              key={kind}
              type="button"
              onClick={() => {
                setOpen(false);
                props.onAdd(make(kind));
              }}
            >
              <Tinted colour={colour}>{icon}</Tinted>
              <span className="text">
                <span className="name">{texts.stepKinds[kind][0]}</span>
                <span className="muted">{texts.stepKinds[kind][1]}</span>
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

/** One sound, picked with a click: the licensed ones first */
function SoundChooser(props: { library: Library | null; onPick: (sound: LibraryItem) => void; onClose: () => void }) {
  const texts = useTexts();
  const [query, setQuery] = useState("");
  const [soundboard, setSoundboard] = useState("");
  const search = query.trim().toLowerCase();
  const sounds = (props.library?.soundboards ?? [])
    .filter((board) => !soundboard || board.id === soundboard)
    .flatMap((board) =>
      board.sounds
        .filter((sound) => !search || sound.name.toLowerCase().includes(search))
        .map((sound) => ({ sound, board: board.name, key: `${board.id}:${sound.id}` }))
    )
    .sort((a, b) => Number(b.sound.enabled) - Number(a.sound.enabled));
  const loaded = (props.library?.soundboards.length ?? 0) > 0;

  return (
    <Dialog label={texts.chooseSound} title={texts.chooseSound} onClose={props.onClose}>
      {loaded && (
        <div className="picker-filters">
          <label className="search">
            <SearchIcon />
            <input
              type="search"
              aria-label={texts.searchSound}
              placeholder={texts.searchSound}
              value={query}
              data-autofocus
              onChange={(event) => setQuery(event.target.value)}
            />
          </label>
          <SearchSelect
            label={texts.soundboard}
            options={[
              { value: "", label: texts.allSoundboards },
              ...(props.library?.soundboards ?? []).map((board) => ({ value: board.id, label: board.name })),
            ]}
            value={soundboard}
            onChange={setSoundboard}
            searchLabel={texts.searchSoundboard}
            empty={texts.noSoundboard}
            align="right"
          />
        </div>
      )}
      <div className="dialog-body">
        {!loaded ? (
          <p className="muted nothing">{texts.voicemodNeeded}</p>
        ) : sounds.length === 0 ? (
          <p className="muted nothing">{texts.noResults}</p>
        ) : (
          <div className="choices">
            {sounds.map(({ sound, board, key }) => (
              <button
                key={key}
                type="button"
                className={sound.enabled ? "choice" : "choice locked"}
                disabled={!sound.enabled}
                title={sound.enabled ? sound.name : texts.locked}
                onClick={() => props.onPick(sound)}
              >
                <PadPicture picture={sound.picture} />
                <span className="text">
                  <span className="name">{sound.name}</span>
                  <span className="muted">{board}</span>
                </span>
              </button>
            ))}
          </div>
        )}
      </div>
    </Dialog>
  );
}
