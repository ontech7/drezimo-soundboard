import { invoke } from "@tauri-apps/api/core";
import { type CSSProperties, type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { type BoardView, showPage } from "../boardView";
import {
  ACTION_COLOURS,
  type Action,
  type CustomPad,
  type CustomPage,
  emptyPage,
  type Library,
  type LibraryItem,
  MACRO_COLOUR,
  MAX_PAGES,
  newId,
  PAD_COUNT,
  padsFrom,
  pageTargets,
  type Rgb,
  rgb,
  shortened,
  SOUND_COLOUR,
  useCustomPages,
  useLibrary,
  VOICE_COLOUR,
} from "../customPages";
import { type Texts, useTexts } from "../i18n";
import { DotsIcon, PlusIcon, PressAgainIcon, TrashIcon } from "../icons";
import { useDismiss } from "../dismiss";
import { ColourPicker, RenameField } from "../editing";
import { type Macro, useMacros } from "../macros";
import { MiniBoard } from "../MiniBoard";
import { PadPicture } from "../PadPicture";
import { type KindMode, type Outcomes, type PlaybackMode, typeChoice, usePressOutcomes } from "../pressAgain";
import { PressAgainField } from "../PressAgainControls";
import { SearchSelect } from "../SearchSelect";
import type { PressAgainByMode, ShownPages } from "../settings";
import { Dialog, Picker, type PickerStart } from "./PadPicker";

const MAX_PAGE_NAME = 24;
const MAX_LABEL = 20;
/** How a pad shows in the editor */
interface Look {
  name: string;
  /** Its own name, without the user's */
  own: string;
  sub: string;
  voice: boolean;
  picture: string | null;
  action: Action | null;
  macro?: boolean;
  colour: Rgb;
  locked: boolean;
  missing: boolean;
}

export interface Lookup {
  /** Voicemod has sent its lists */
  loaded: boolean;
  macro: (id: string) => Macro | undefined;
  sound: (id: string) => { item: LibraryItem; soundboard: string } | undefined;
  voice: (id: string) => LibraryItem | undefined;
  /** The pages a pad can open */
  targets: { id: string; name: string }[];
}

function useLookup(library: Library | null, pages: CustomPage[], macros: Macro[], shown: ShownPages, texts: Texts): Lookup {
  return useMemo(() => {
    const sounds = new Map<string, { item: LibraryItem; soundboard: string }>();
    for (const soundboard of library?.soundboards ?? []) {
      for (const item of soundboard.sounds) {
        if (!sounds.has(item.id)) sounds.set(item.id, { item, soundboard: soundboard.name });
      }
    }
    const voices = new Map((library?.voices ?? []).map((voice) => [voice.id, voice]));
    const targets = pageTargets(pages, library, shown, texts);
    return {
      loaded: (library?.soundboards.length ?? 0) > 0,
      macro: (id) => macros.find((item) => item.id === id),
      sound: (id) => sounds.get(id),
      voice: (id) => voices.get(id),
      targets,
    };
  }, [library, pages, macros, shown, texts]);
}

function look(pad: CustomPad, lookup: Lookup, texts: Texts): Look {
  const shown = (own: string) => pad.label || own;
  switch (pad.kind) {
    case "sound": {
      const found = lookup.sound(pad.id);
      const own = found?.item.name ?? pad.name;
      return {
        own,
        name: shown(own),
        sub: found ? texts.soundOf(found.soundboard) : lookup.loaded ? texts.missing : "",
        voice: false,
        picture: found?.item.picture ?? null,
        action: null,
        colour: pad.colour ?? SOUND_COLOUR,
        locked: found ? !found.item.enabled : false,
        missing: lookup.loaded && !found,
      };
    }
    case "voice": {
      const found = lookup.voice(pad.id);
      const own = found?.name ?? pad.name;
      return {
        own,
        name: shown(own),
        sub: found ? texts.voicemodVoice : lookup.loaded ? texts.missing : "",
        voice: true,
        picture: found?.picture ?? null,
        action: null,
        colour: pad.colour ?? VOICE_COLOUR,
        locked: found ? !found.enabled : false,
        missing: lookup.loaded && !found,
      };
    }
    case "macro": {
      const found = lookup.macro(pad.id);
      const own = found?.name ?? "";
      return {
        own,
        name: shown(own),
        sub: found ? texts.macroKind : texts.macroDeleted,
        voice: false,
        picture: null,
        action: null,
        macro: true,
        colour: pad.colour ?? found?.colour ?? MACRO_COLOUR,
        locked: false,
        missing: !found,
      };
    }
    case "action": {
      const target = pad.action === "goToPage" ? lookup.targets.find((page) => page.id === pad.page) : undefined;
      const own = target ? texts.goTo(target.name) : texts.actionNames[pad.action];
      return {
        own,
        name: shown(own),
        sub: texts.actionTexts[pad.action],
        voice: false,
        picture: null,
        action: pad.action,
        colour: pad.colour ?? ACTION_COLOURS[pad.action],
        locked: false,
        missing: false,
      };
    }
  }
}

interface PagesProps {
  /** The board as the app drives it: it shows the page open here */
  view: BoardView | null;
  /** The page to open first, if the user has it */
  open: string | null;
  shownPages: ShownPages;
  /** As set on the Drezimo page */
  pressAgainByMode: PressAgainByMode;
  /** Voicemod's bridge was found: a pad can stop a single sound */
  singleStop: boolean;
  onOpenMacros: () => void;
}

export function PagesPage({ view, open, shownPages, pressAgainByMode, singleStop, onOpenMacros }: PagesProps) {
  const texts = useTexts();
  const [pages, save, saveError] = useCustomPages();
  const library = useLibrary();
  const outcomes = usePressOutcomes(singleStop);
  const [current, setCurrent] = useState(open);
  const [selected, setSelected] = useState<number | null>(null);
  const [picking, setPicking] = useState<PickerStart | null>(null);
  const [startingFrom, setStartingFrom] = useState(false);
  // A pad being dragged: the space below the page becomes a bin for it
  const [dragging, setDragging] = useState<number | null>(null);
  const [macros] = useMacros();
  const lookup = useLookup(library, pages ?? [], macros ?? [], shownPages, texts);
  const shownId = pages?.find((page) => page.id === current)?.id ?? pages?.[0]?.id ?? null;
  useBoardFollows(view, shownId);

  if (!pages) return null;

  const add = (page: CustomPage) => {
    save([...pages, page]);
    setCurrent(page.id);
    setSelected(null);
  };
  const chooser = startingFrom && (
    <SoundboardChooser
      library={library}
      onPick={(page) => {
        setStartingFrom(false);
        add(page);
      }}
      onClose={() => setStartingFrom(false)}
    />
  );

  if (pages.length === 0) {
    return (
      <>
        <EmptyPages onBlank={() => add(emptyPage(texts.pageNumber(1)))} onFromSoundboard={() => setStartingFrom(true)} />
        {chooser}
      </>
    );
  }

  const index = Math.max(
    0,
    pages.findIndex((page) => page.id === shownId)
  );
  const page = pages[index];
  const savePage = (next: CustomPage) => save(pages.map((other, i) => (i === index ? next : other)));
  const setPad = (pad: number, value: CustomPad | null) =>
    savePage({ ...page, pads: page.pads.map((other, i) => (i === pad ? value : other)) });
  const selectedPad = selected === null ? null : page.pads[selected];

  const place = (start: number, contents: CustomPad[]) => {
    const pads = [...page.pads];
    const slots = padsFrom(pads, start);
    contents.forEach((content, k) => {
      const at = slots[k];
      if (at === undefined) return;
      // A replaced pad keeps the colour picked for it, and a sound what it does when pressed again
      const before = pads[at];
      const pressAgain = content.kind === "sound" && before?.kind === "sound" ? before.pressAgain : null;
      pads[at] = { ...content, colour: content.colour ?? before?.colour ?? null, pressAgain };
    });
    savePage({ ...page, pads });
    setSelected(start);
    setPicking(null);
  };

  const clear = (pad: number) => {
    setPad(pad, null);
    setSelected((shown) => (shown === pad ? null : shown));
  };

  const move = (from: number, to: number) => {
    if (from === to) return;
    const pads = [...page.pads];
    [pads[from], pads[to]] = [pads[to], pads[from]];
    savePage({ ...page, pads });
    setSelected((shown) => (shown === from ? to : shown === to ? from : shown));
  };

  return (
    <>
      {saveError && <p className="note">{texts.saveFailed(saveError)}</p>}
      <section className="card editor" aria-label={page.name}>
        <PageTabs
          pages={pages}
          index={index}
          used={page.pads.filter(Boolean).length}
          onSelect={(i) => {
            setCurrent(pages[i].id);
            setSelected(null);
          }}
          onAdd={() => add(emptyPage(texts.pageNumber(pages.length + 1)))}
          onAddFrom={() => setStartingFrom(true)}
          onRename={(name) => savePage({ ...page, name })}
          onDuplicate={() => {
            const copy = { ...page, id: newId(), name: shortened(texts.copyOf(page.name), MAX_PAGE_NAME) };
            save([...pages.slice(0, index + 1), copy, ...pages.slice(index + 1)]);
            setCurrent(copy.id);
          }}
          onMove={(step) => {
            const to = index + step;
            if (to < 0 || to >= pages.length) return;
            const next = [...pages];
            [next[index], next[to]] = [next[to], next[index]];
            save(next);
          }}
          onDelete={() => {
            save(pages.filter((_, i) => i !== index));
            setCurrent(pages[Math.max(0, index - 1)].id);
            setSelected(null);
          }}
        />
        <PadGrid
          page={page}
          lookup={lookup}
          selected={selected}
          onSelect={setSelected}
          onOpen={(pad) => setPicking({ pad, tab: "sounds", chosen: [], replace: false })}
          onReplace={(pad) => {
            const content = page.pads[pad];
            if (content) setPicking(pickerStart(pad, content));
          }}
          onMove={move}
          onClear={clear}
          dragging={dragging !== null}
          onDragging={setDragging}
        />
      </section>

      {dragging !== null ? (
        <Bin
          onDrop={() => {
            clear(dragging);
            setDragging(null);
          }}
        />
      ) : selectedPad && selected !== null ? (
        <Inspector
          pad={selectedPad}
          look={look(selectedPad, lookup, texts)}
          pressAgainByMode={pressAgainByMode}
          mode={selectedPad.kind === "sound" ? (lookup.sound(selectedPad.id)?.item.mode ?? null) : null}
          outcomes={outcomes}
          targets={lookup.targets.filter((target) => target.id !== page.id)}
          singleStop={singleStop}
          onChange={(pad) => setPad(selected, pad)}
          onReplace={() => setPicking(pickerStart(selected, selectedPad))}
          onClear={() => clear(selected)}
        />
      ) : (
        <p className="editor-hint">{texts.editorHint}</p>
      )}

      {picking && (
        <Picker
          page={page}
          start={picking}
          library={library}
          lookup={lookup}
          macros={macros ?? []}
          onOpenMacros={onOpenMacros}
          onPlace={place}
          onClose={() => setPicking(null)}
        />
      )}
      {chooser}
    </>
  );
}

/**
 * The board shows the page open in the editor, to try it there; once there, the board is free to move on. A new page
 * gets there as soon as the board has it.
 */
function useBoardFollows(view: BoardView | null, id: string | null) {
  const latest = useRef(view);
  latest.current = view;
  const available = !!id && !!view?.ids.includes(id);
  useEffect(() => {
    if (id && available && latest.current) showPage(latest.current, id);
  }, [id, available]);
}

/** The picker opened on a full pad, on its kind, with it chosen. */
function pickerStart(pad: number, content: CustomPad): PickerStart {
  switch (content.kind) {
    case "sound":
      return { pad, tab: "sounds", chosen: [`sound:${content.id}`], replace: true };
    case "voice":
      return { pad, tab: "voices", chosen: [`voice:${content.id}`], replace: true };
    case "action":
      return { pad, tab: "actions", chosen: [`action:${content.action}`], replace: true };
    case "macro":
      return { pad, tab: "macros", chosen: [`macro:${content.id}`], replace: true };
  }
}

function EmptyPages({ onBlank, onFromSoundboard }: { onBlank: () => void; onFromSoundboard: () => void }) {
  const texts = useTexts();
  // A board with a few pads lit, as a page of the user's own could look
  const lit: Record<number, Rgb> = {
    0: SOUND_COLOUR,
    1: SOUND_COLOUR,
    2: SOUND_COLOUR,
    3: SOUND_COLOUR,
    6: SOUND_COLOUR,
    7: SOUND_COLOUR,
    18: VOICE_COLOUR,
    19: VOICE_COLOUR,
    20: VOICE_COLOUR,
    30: ACTION_COLOURS.muteMic,
    31: ACTION_COLOURS.muteForMe,
    32: ACTION_COLOURS.voiceChanger,
  };
  return (
    <section className="card empty-pages">
      <MiniBoard leds={Array.from({ length: PAD_COUNT }, (_, i) => lit[i] ?? null)} />
      <h2>{texts.emptyPagesTitle}</h2>
      <p className="muted">{texts.emptyPagesText}</p>
      <div className="actions">
        <button type="button" className="button" onClick={onBlank}>
          {texts.newPage}
        </button>
        <button type="button" className="button secondary" onClick={onFromSoundboard}>
          {texts.fromSoundboard}
        </button>
      </div>
    </section>
  );
}

interface TabsProps {
  pages: CustomPage[];
  index: number;
  used: number;
  onSelect: (index: number) => void;
  onAdd: () => void;
  onAddFrom: () => void;
  onRename: (name: string) => void;
  onDuplicate: () => void;
  onMove: (step: number) => void;
  onDelete: () => void;
}

function PageTabs(props: TabsProps) {
  const texts = useTexts();
  const [renaming, setRenaming] = useState(false);
  const [adding, setAdding] = useState(false);
  const [menu, setMenu] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const addMenu = useDismiss<HTMLDivElement>(adding, () => setAdding(false));
  const pageMenu = useDismiss<HTMLDivElement>(menu, () => {
    setMenu(false);
    setConfirming(false);
  });
  const run = (action: () => void) => () => {
    setMenu(false);
    setConfirming(false);
    action();
  };
  const full = props.pages.length >= MAX_PAGES;

  return (
    <div className="page-tabs">
      <div className="tabs" role="group" aria-label={texts.yourPages}>
        {props.pages.map((page, i) =>
          renaming && i === props.index ? (
            <RenameField
              key={page.id}
              name={page.name}
              label={texts.pageName}
              max={MAX_PAGE_NAME}
              onDone={(name) => {
                setRenaming(false);
                if (name) props.onRename(name);
              }}
            />
          ) : (
            <button
              key={page.id}
              type="button"
              className={i === props.index ? "chip selected" : "chip"}
              aria-pressed={i === props.index}
              onClick={() => props.onSelect(i)}
              onDoubleClick={() => i === props.index && setRenaming(true)}
            >
              {page.name}
            </button>
          )
        )}
      </div>
      <div className="anchor" ref={addMenu}>
        <button
          type="button"
          className="chip add"
          aria-label={texts.newPage}
          title={full ? texts.tooManyPages : texts.newPage}
          aria-expanded={adding}
          disabled={full}
          onClick={() => setAdding(!adding)}
        >
          <PlusIcon />
        </button>
        {adding && (
          <div className="menu">
            <button
              type="button"
              onClick={() => {
                setAdding(false);
                props.onAdd();
              }}
            >
              {texts.blankPage}
            </button>
            <button
              type="button"
              onClick={() => {
                setAdding(false);
                props.onAddFrom();
              }}
            >
              {texts.fromSoundboard}…
            </button>
          </div>
        )}
      </div>
      <span className="spacer" />
      <span className="muted used">{texts.padsUsed(props.used)}</span>
      <div className="anchor" ref={pageMenu}>
        <button
          type="button"
          className="icon-button small"
          aria-label={texts.pageOptions}
          title={texts.pageOptions}
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
            <button type="button" disabled={props.index === 0} onClick={run(() => props.onMove(-1))}>
              {texts.moveLeft}
            </button>
            <button type="button" disabled={props.index === props.pages.length - 1} onClick={run(() => props.onMove(1))}>
              {texts.moveRight}
            </button>
            {/* Deleting takes a second click: the pads go with the page */}
            <button type="button" className="danger" onClick={confirming ? run(props.onDelete) : () => setConfirming(true)}>
              {confirming ? `${texts.deletePage} “${props.pages[props.index].name}”?` : texts.deletePage}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

/** With what it does when pressed again, where the pad has its own */
function padLabel(texts: Texts, number: number, name: string, pad: CustomPad | null): string {
  const label = texts.padLabel(number, name);
  return pad?.kind === "sound" && pad.pressAgain
    ? `${label}, ${texts.ownPressAgain(texts.pressAgainNames[pad.pressAgain])}`
    : label;
}

interface GridProps {
  page: CustomPage;
  lookup: Lookup;
  selected: number | null;
  onSelect: (pad: number) => void;
  onOpen: (pad: number) => void;
  onReplace: (pad: number) => void;
  onMove: (from: number, to: number) => void;
  onClear: (pad: number) => void;
  dragging: boolean;
  /** The pad being dragged, or null once let go */
  onDragging: (pad: number | null) => void;
}

function PadGrid(props: GridProps) {
  const { page, lookup, selected, onSelect, onOpen, onReplace, onMove, onClear, dragging, onDragging } = props;
  const texts = useTexts();
  const dragged = useRef<number | null>(null);
  const [over, setOver] = useState<number | null>(null);
  const ended = () => {
    dragged.current = null;
    setOver(null);
    onDragging(null);
  };

  return (
    <div className="pads">
      {page.pads.map((pad, i) => {
        const shown = pad && look(pad, lookup, texts);
        const classes = [
          "pad",
          shown ? "lit" : "empty",
          i === selected ? "selected" : "",
          i === over ? "over" : "",
          shown?.locked || shown?.missing ? "locked" : "",
        ];
        return (
          <button
            key={i}
            type="button"
            className={classes.join(" ")}
            // Placed in its cell, as the selected pad's menu shares it
            style={{ ...cell(i), ...(shown ? { "--led": rgb(shown.colour) } : {}) } as CSSProperties}
            aria-label={shown ? padLabel(texts, i + 1, shown.name, pad) : texts.emptyPadLabel(i + 1)}
            aria-pressed={shown ? i === selected : undefined}
            title={shown ? `${shown.name} · ${texts.dragHint}` : undefined}
            draggable={!!shown}
            onClick={() => (shown ? onSelect(i) : onOpen(i))}
            onKeyDown={(event) => {
              if (shown && (event.key === "Delete" || event.key === "Backspace")) onClear(i);
            }}
            onDragStart={(event) => {
              dragged.current = i;
              event.dataTransfer.effectAllowed = "move";
              onDragging(i);
            }}
            onDragOver={(event) => {
              if (dragged.current === null) return;
              event.preventDefault();
              setOver(i);
            }}
            onDragLeave={() => setOver((shownOver) => (shownOver === i ? null : shownOver))}
            onDrop={(event) => {
              event.preventDefault();
              if (dragged.current !== null) onMove(dragged.current, i);
              ended();
            }}
            onDragEnd={ended}
          >
            {shown ? (
              <>
                <PadPicture
                  voice={shown.voice}
                  picture={shown.picture}
                  action={shown.action}
                  macro={shown.macro}
                  tint={shown.colour}
                />
                <span className="name">{shown.name}</span>
                {pad.kind === "sound" && pad.pressAgain && (
                  <span className="own-press" title={texts.ownPressAgain(texts.pressAgainNames[pad.pressAgain])}>
                    <PressAgainIcon choice={pad.pressAgain} />
                  </span>
                )}
              </>
            ) : (
              <PlusIcon />
            )}
          </button>
        );
      })}
      {/* Out of the way of a pad dropped on its cell */}
      {!dragging && selected !== null && page.pads[selected] && (
        <PadMenu key={selected} pad={selected} onReplace={() => onReplace(selected)} onClear={() => onClear(selected)} />
      )}
    </div>
  );
}

const COLUMNS = 6;

function cell(pad: number): CSSProperties {
  return { gridRow: Math.floor(pad / COLUMNS) + 1, gridColumn: (pad % COLUMNS) + 1 };
}

/** Over the selected pad's top right corner, in its own cell of the grid: what the bar below the page also does */
function PadMenu({ pad, onReplace, onClear }: { pad: number; onReplace: () => void; onClear: () => void }) {
  const texts = useTexts();
  const [open, setOpen] = useState(false);
  const menu = useDismiss<HTMLDivElement>(open, () => setOpen(false));
  const run = (action: () => void) => () => {
    setOpen(false);
    action();
  };
  return (
    <div className={open ? "pad-menu anchor open" : "pad-menu anchor"} style={cell(pad)} ref={menu}>
      <button
        type="button"
        className="pad-menu-button"
        aria-label={texts.padOptions}
        title={texts.padOptions}
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        <DotsIcon />
      </button>
      {open && (
        <div className={pad % COLUMNS < COLUMNS / 2 ? "menu" : "menu right"}>
          <button type="button" onClick={run(onReplace)}>
            {texts.change}
          </button>
          <button type="button" className="danger" onClick={run(onClear)}>
            {texts.clearPad}
          </button>
        </div>
      )}
    </div>
  );
}

/** Where a pad dragged off the page is cleared: shown only while one is dragged */
function Bin({ onDrop }: { onDrop: () => void }) {
  const texts = useTexts();
  const [over, setOver] = useState(false);
  return (
    <div
      className={over ? "bin over" : "bin"}
      onDragOver={(event) => {
        event.preventDefault();
        event.dataTransfer.dropEffect = "move";
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(event) => {
        event.preventDefault();
        onDrop();
      }}
    >
      <TrashIcon />
      {texts.dropToClear}
    </div>
  );
}

interface InspectorProps {
  pad: CustomPad;
  look: Look;
  /** As set on the Drezimo page */
  pressAgainByMode: PressAgainByMode;
  mode: PlaybackMode | null;
  outcomes: Outcomes | null;
  targets: { id: string; name: string }[];
  /** Voicemod's bridge was found: a sound's mode can be changed */
  singleStop: boolean;
  onChange: (pad: CustomPad) => void;
  onReplace: () => void;
  onClear: () => void;
}

function Inspector({
  pad,
  look: shown,
  pressAgainByMode,
  mode,
  outcomes,
  targets,
  singleStop,
  onChange,
  onReplace,
  onClear,
}: InspectorProps) {
  const texts = useTexts();

  return (
    <section className="card inspector" aria-label={shown.name}>
      <PadPicture
        voice={shown.voice}
        picture={shown.picture}
        action={shown.action}
        macro={shown.macro}
        tint={shown.colour}
        large
      />
      <div className="identity">
        <p className="title">{shown.name}</p>
        <p className="muted">{shown.sub}</p>
      </div>
      <label className="field">
        {texts.nameOnDisplay}
        <input
          type="text"
          value={pad.label}
          placeholder={shown.own}
          maxLength={MAX_LABEL}
          onChange={(event) => onChange({ ...pad, label: event.target.value })}
        />
      </label>
      {pad.kind === "action" && pad.action === "goToPage" && (
        <div className="field">
          {texts.pageToOpen}
          {/* A page gone, or none picked yet, shows as the placeholder */}
          <SearchSelect
            label={texts.pageToOpen}
            options={targets.map((target) => ({ value: target.id, label: target.name }))}
            value={pad.page ?? ""}
            placeholder={texts.choosePage}
            onChange={(page) => onChange({ ...pad, page })}
            searchLabel={texts.searchPage}
            empty={texts.noPage}
          />
        </div>
      )}
      {pad.kind === "sound" && mode && mode !== "Unknown" && (
        <ModeField key={pad.id} sound={pad.id} mode={mode} bridge={singleStop} />
      )}
      {pad.kind === "sound" && (
        <PressAgainField
          own={pad.pressAgain ?? null}
          typeChoice={typeChoice(pressAgainByMode, mode)}
          mode={mode}
          outcomes={outcomes}
          onChange={(pressAgain) => onChange({ ...pad, pressAgain })}
        />
      )}
      <ColourPicker colour={pad.colour} shown={shown.colour} onChange={(colour) => onChange({ ...pad, colour })} />
      <button type="button" className="button secondary" onClick={onReplace}>
        {texts.change}
      </button>
      <button type="button" className="icon-button" aria-label={texts.clearPad} title={texts.clearPad} onClick={onClear}>
        <TrashIcon />
      </button>
    </section>
  );
}

const MODES: KindMode[] = ["PlayRestart", "PlayOverlap", "PlayStop", "PlayPause", "PlayLoopOnPress"];
/** Voicemod takes about a second to change it and send the soundboards again */
const MODE_WAIT = 4000;

/** The sound's own mode in Voicemod, changed there for good: on every soundboard that has it */
function ModeField({ sound, mode, bridge }: { sound: string; mode: KindMode; bridge: boolean }) {
  const texts = useTexts();
  const [asked, setAsked] = useState<KindMode | null>(null);

  useEffect(() => {
    if (!asked) return;
    const timer = setTimeout(() => setAsked(null), MODE_WAIT);
    return () => clearTimeout(timer);
  }, [asked]);
  useEffect(() => setAsked(null), [mode]);

  return (
    <div className="field" title={texts.modeInVoicemodHint}>
      {texts.modeInVoicemod}
      {bridge ? (
        <SearchSelect
          label={texts.modeInVoicemod}
          options={MODES.map((value) => ({ value, label: texts.modeNames[value] }))}
          value={asked ?? mode}
          onChange={(picked) => {
            if (picked === (asked ?? mode)) return;
            setAsked(picked);
            invoke("set_sound_mode", { id: sound, mode: picked }).catch((error) =>
              console.error("cannot change the sound's mode", error)
            );
          }}
        />
      ) : (
        <span className="field-value" title={texts.modeNeedsBridge}>
          {texts.modeNames[mode]}
        </span>
      )}
    </div>
  );
}

function SoundboardChooser(props: { library: Library | null; onPick: (page: CustomPage) => void; onClose: () => void }) {
  const texts = useTexts();
  const soundboards = props.library?.soundboards.filter((soundboard) => soundboard.sounds.length > 0) ?? [];
  const pick = (soundboard: Library["soundboards"][number]) => {
    const pads: (CustomPad | null)[] = soundboard.sounds
      .slice(0, PAD_COUNT)
      .map((sound) => ({ kind: "sound", id: sound.id, name: sound.name, label: "", colour: null }));
    while (pads.length < PAD_COUNT) pads.push(null);
    props.onPick({ id: newId(), name: shortened(soundboard.name, MAX_PAGE_NAME), pads });
  };
  let list: ReactNode;
  if (soundboards.length === 0) {
    list = <p className="muted nothing">{texts.voicemodNeeded}</p>;
  } else {
    list = (
      <div className="choices">
        {soundboards.map((soundboard) => (
          <button key={soundboard.id} type="button" className="choice" onClick={() => pick(soundboard)}>
            <PadPicture picture={soundboard.sounds[0]?.picture} />
            <span className="text">
              <span className="name">{soundboard.name}</span>
              <span className="muted">{texts.soundCount(soundboard.sounds.length)}</span>
            </span>
          </button>
        ))}
      </div>
    );
  }
  return (
    <Dialog
      label={texts.fromSoundboard}
      title={texts.fromSoundboard}
      subtitle={texts.fromSoundboardText}
      onClose={props.onClose}
    >
      <div className="dialog-body">{list}</div>
    </Dialog>
  );
}
