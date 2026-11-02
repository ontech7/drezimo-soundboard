import { type CSSProperties, type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import {
  ACTION_COLOURS,
  type Action,
  type CustomPad,
  type CustomPage,
  emptyPage,
  FAVOURITES_ID,
  type Library,
  type LibraryItem,
  MAX_PAGES,
  newId,
  PAD_COUNT,
  padsFrom,
  type Rgb,
  rgb,
  shortened,
  SOUND_COLOUR,
  useCustomPages,
  useLibrary,
  VOICE_COLOUR,
  VOICES_ID,
} from "../customPages";
import { type Texts, useTexts } from "../i18n";
import { ChevronDown, DotsIcon, PlusIcon, TrashIcon } from "../icons";
import { PadPicture } from "../PadPicture";
import { Dialog, Picker, type PickerStart } from "./PadPicker";

const MAX_PAGE_NAME = 24;
const MAX_LABEL = 20;
/** The LEDs' colours the user can pick, the kind's own first */
const SWATCHES: (Rgb | null)[] = [
  null,
  [47, 107, 255],
  [150, 80, 255],
  [34, 224, 90],
  [40, 200, 255],
  [255, 210, 26],
  [255, 140, 0],
  [255, 42, 42],
  [255, 60, 160],
  [200, 200, 210],
];

/** How a pad shows in the editor */
interface Look {
  name: string;
  /** Its own name, without the user's */
  own: string;
  sub: string;
  voice: boolean;
  picture: string | null;
  action: Action | null;
  colour: Rgb;
  locked: boolean;
  missing: boolean;
}

export interface Lookup {
  /** Voicemod has sent its lists */
  loaded: boolean;
  sound: (id: string) => { item: LibraryItem; soundboard: string } | undefined;
  voice: (id: string) => LibraryItem | undefined;
  /** The pages a pad can open */
  targets: { id: string; name: string }[];
}

function useLookup(library: Library | null, pages: CustomPage[], texts: Texts): Lookup {
  return useMemo(() => {
    const sounds = new Map<string, { item: LibraryItem; soundboard: string }>();
    for (const soundboard of library?.soundboards ?? []) {
      for (const item of soundboard.sounds) {
        if (!sounds.has(item.id)) sounds.set(item.id, { item, soundboard: soundboard.name });
      }
    }
    const voices = new Map((library?.voices ?? []).map((voice) => [voice.id, voice]));
    const targets = [
      ...pages.map((page) => ({ id: page.id, name: page.name })),
      // As the board has them
      ...(library?.favourites ? [{ id: FAVOURITES_ID, name: texts.favouritesPage }] : []),
      ...(library?.voices.some((voice) => voice.enabled) ? [{ id: VOICES_ID, name: texts.voicesPage }] : []),
      ...(library?.soundboards ?? []).map((soundboard) => ({ id: soundboard.id, name: soundboard.name })),
    ];
    return {
      loaded: (library?.soundboards.length ?? 0) > 0,
      sound: (id) => sounds.get(id),
      voice: (id) => voices.get(id),
      targets,
    };
  }, [library, pages, texts]);
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

/** Closes a menu or popover on a click elsewhere or Escape. */
function useDismiss<T extends HTMLElement>(open: boolean, close: () => void) {
  const ref = useRef<T>(null);
  const closing = useRef(close);
  closing.current = close;
  useEffect(() => {
    if (!open) return;
    const away = (event: PointerEvent) => {
      if (!ref.current?.contains(event.target as Node)) closing.current();
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") closing.current();
    };
    document.addEventListener("pointerdown", away);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("pointerdown", away);
      document.removeEventListener("keydown", escape);
    };
  }, [open]);
  return ref;
}

export function PagesPage() {
  const texts = useTexts();
  const [pages, save, saveError] = useCustomPages();
  const library = useLibrary();
  const [current, setCurrent] = useState(0);
  const [selected, setSelected] = useState<number | null>(null);
  const [picking, setPicking] = useState<PickerStart | null>(null);
  const [startingFrom, setStartingFrom] = useState(false);
  const lookup = useLookup(library, pages ?? [], texts);

  if (!pages) return null;

  const add = (page: CustomPage) => {
    save([...pages, page]);
    setCurrent(pages.length);
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

  const index = Math.min(current, pages.length - 1);
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
      // A replaced pad keeps the colour picked for it
      pads[at] = { ...content, colour: content.colour ?? pads[at]?.colour ?? null };
    });
    savePage({ ...page, pads });
    setSelected(start);
    setPicking(null);
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
            setCurrent(i);
            setSelected(null);
          }}
          onAdd={() => add(emptyPage(texts.pageNumber(pages.length + 1)))}
          onAddFrom={() => setStartingFrom(true)}
          onRename={(name) => savePage({ ...page, name })}
          onDuplicate={() => {
            const copy = { ...page, id: newId(), name: shortened(texts.copyOf(page.name), MAX_PAGE_NAME) };
            save([...pages.slice(0, index + 1), copy, ...pages.slice(index + 1)]);
            setCurrent(index + 1);
          }}
          onMove={(step) => {
            const to = index + step;
            if (to < 0 || to >= pages.length) return;
            const next = [...pages];
            [next[index], next[to]] = [next[to], next[index]];
            save(next);
            setCurrent(to);
          }}
          onDelete={() => {
            save(pages.filter((_, i) => i !== index));
            setCurrent(Math.max(0, index - 1));
            setSelected(null);
          }}
        />
        <PadGrid
          page={page}
          lookup={lookup}
          selected={selected}
          onSelect={setSelected}
          onOpen={(pad) => setPicking({ pad, tab: "sounds", chosen: [], replace: false })}
          onMove={move}
          onClear={(pad) => {
            setPad(pad, null);
            setSelected(null);
          }}
        />
      </section>

      {selectedPad && selected !== null ? (
        <Inspector
          pad={selectedPad}
          look={look(selectedPad, lookup, texts)}
          targets={lookup.targets.filter((target) => target.id !== page.id)}
          onChange={(pad) => setPad(selected, pad)}
          onReplace={() => setPicking(pickerStart(selected, selectedPad))}
          onClear={() => {
            setPad(selected, null);
            setSelected(null);
          }}
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
          onPlace={place}
          onClose={() => setPicking(null)}
        />
      )}
      {chooser}
    </>
  );
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
      <div className="mini-board" aria-hidden="true">
        {Array.from({ length: PAD_COUNT }, (_, i) => (
          <span
            key={i}
            className={lit[i] ? "lit" : undefined}
            style={lit[i] ? ({ "--led": rgb(lit[i]) } as CSSProperties) : undefined}
          />
        ))}
      </div>
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

function RenameField({ name, onDone }: { name: string; onDone: (name: string) => void }) {
  const texts = useTexts();
  const [value, setValue] = useState(name);
  const done = useRef(false);
  const finish = (result: string) => {
    if (done.current) return;
    done.current = true;
    onDone(result.trim());
  };
  return (
    <input
      className="chip rename"
      aria-label={texts.pageName}
      value={value}
      maxLength={MAX_PAGE_NAME}
      autoFocus
      onFocus={(event) => event.currentTarget.select()}
      onChange={(event) => setValue(event.target.value)}
      onBlur={() => finish(value)}
      onKeyDown={(event) => {
        if (event.key === "Enter") finish(value);
        if (event.key === "Escape") finish("");
      }}
    />
  );
}

interface GridProps {
  page: CustomPage;
  lookup: Lookup;
  selected: number | null;
  onSelect: (pad: number) => void;
  onOpen: (pad: number) => void;
  onMove: (from: number, to: number) => void;
  onClear: (pad: number) => void;
}

function PadGrid({ page, lookup, selected, onSelect, onOpen, onMove, onClear }: GridProps) {
  const texts = useTexts();
  const dragged = useRef<number | null>(null);
  const [over, setOver] = useState<number | null>(null);

  return (
    <div className="pads">
      {page.pads.map((pad, i) => {
        const shown = pad && look(pad, lookup, texts);
        const classes = [
          "pad",
          "edit",
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
            style={shown ? ({ "--led": rgb(shown.colour) } as CSSProperties) : undefined}
            aria-label={shown ? texts.padLabel(i + 1, shown.name) : texts.emptyPadLabel(i + 1)}
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
              dragged.current = null;
              setOver(null);
            }}
            onDragEnd={() => {
              dragged.current = null;
              setOver(null);
            }}
          >
            {shown ? (
              <>
                <PadPicture voice={shown.voice} picture={shown.picture} action={shown.action} />
                <span className="name">{shown.name}</span>
              </>
            ) : (
              <PlusIcon />
            )}
          </button>
        );
      })}
    </div>
  );
}

interface InspectorProps {
  pad: CustomPad;
  look: Look;
  targets: { id: string; name: string }[];
  onChange: (pad: CustomPad) => void;
  onReplace: () => void;
  onClear: () => void;
}

function Inspector({ pad, look: shown, targets, onChange, onReplace, onClear }: InspectorProps) {
  const texts = useTexts();
  const [colours, setColours] = useState(false);
  const popover = useDismiss<HTMLDivElement>(colours, () => setColours(false));

  return (
    <section className="card inspector" aria-label={shown.name}>
      <PadPicture voice={shown.voice} picture={shown.picture} action={shown.action} large />
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
        <label className="field">
          {texts.pageToOpen}
          <select value={pad.page ?? ""} onChange={(event) => onChange({ ...pad, page: event.target.value })}>
            {/* A page gone, or none picked yet */}
            {!targets.some((target) => target.id === pad.page) && (
              <option value={pad.page ?? ""} disabled>
                {texts.choosePage}
              </option>
            )}
            {targets.map((target) => (
              <option key={target.id} value={target.id}>
                {target.name}
              </option>
            ))}
          </select>
        </label>
      )}
      <div className="anchor" ref={popover}>
        <button
          type="button"
          className="colour-button"
          aria-label={texts.ledColour}
          title={texts.ledColour}
          aria-expanded={colours}
          onClick={() => setColours(!colours)}
        >
          <span className="dot" style={{ "--led": rgb(shown.colour) } as CSSProperties} />
          <ChevronDown />
        </button>
        {colours && (
          <div className="popover" role="dialog" aria-label={texts.ledAtRest}>
            <p className="muted">{texts.ledAtRest}</p>
            <div className="swatches">
              {SWATCHES.map((colour, i) => {
                const name = colour ? texts.colourNames[i - 1] : texts.automatic;
                const on = colour ? pad.colour?.join() === colour.join() : pad.colour === null;
                return (
                  <button
                    key={i}
                    type="button"
                    className={on ? "swatch on" : "swatch"}
                    aria-label={name}
                    title={name}
                    aria-pressed={on}
                    style={colour ? ({ "--led": rgb(colour) } as CSSProperties) : undefined}
                    onClick={() => onChange({ ...pad, colour })}
                  >
                    {colour ? null : "Auto"}
                  </button>
                );
              })}
            </div>
          </div>
        )}
      </div>
      <button type="button" className="button secondary" onClick={onReplace}>
        {texts.change}
      </button>
      <button type="button" className="icon-button" aria-label={texts.clearPad} title={texts.clearPad} onClick={onClear}>
        <TrashIcon />
      </button>
    </section>
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
