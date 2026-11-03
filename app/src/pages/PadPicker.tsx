import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import {
  ACTIONS,
  type Action,
  type CustomPad,
  type CustomPage,
  type Library,
  MACRO_COLOUR,
  padsFrom,
  type Rgb,
} from "../customPages";
import { useTexts } from "../i18n";
import { CheckIcon, CloseIcon, LockIcon, PlusIcon, SearchIcon } from "../icons";
import { type Macro, stepLength } from "../macros";
import { PadPicture } from "../PadPicture";
import { SearchSelect } from "../SearchSelect";
import type { Lookup } from "./PagesPage";

export type Tab = "sounds" | "voices" | "actions" | "macros";

export interface PickerStart {
  pad: number;
  tab: Tab;
  /** "sound:<id>", "voice:<id>" or "action:<action>" */
  chosen: string[];
  /** Changing a full pad: one choice, which replaces it */
  replace: boolean;
}

function keyOf(pad: CustomPad): string {
  return pad.kind === "action" ? `action:${pad.action}` : `${pad.kind}:${pad.id}`;
}

interface Choice {
  key: string;
  /** Unique in the list: a sound can sit in more than one soundboard */
  id: string;
  name: string;
  sub: string;
  voice?: boolean;
  picture?: string | null;
  action?: Action;
  macro?: boolean;
  tint?: Rgb;
  locked: boolean;
}

/** A modal dialog: Escape and a click beside it close it. */
export function Dialog(props: { label: string; title: string; subtitle?: string; onClose: () => void; children: ReactNode }) {
  const texts = useTexts();
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    // Focus goes back where it was when the dialog goes
    const opener = document.activeElement as HTMLElement | null;
    dialog.current?.showModal();
    dialog.current?.querySelector<HTMLElement>("[data-autofocus]")?.focus();
    return () => opener?.focus();
  }, []);
  return (
    <dialog
      ref={dialog}
      className="dialog"
      aria-label={props.label}
      onClose={props.onClose}
      onClick={(event) => {
        if (event.target === dialog.current) props.onClose();
      }}
    >
      <header>
        <div>
          <h2>{props.title}</h2>
          {props.subtitle && <p className="muted">{props.subtitle}</p>}
        </div>
        <button
          type="button"
          className="icon-button small"
          aria-label={texts.close}
          title={texts.close}
          onClick={props.onClose}
        >
          <CloseIcon />
        </button>
      </header>
      {props.children}
    </dialog>
  );
}

interface PickerProps {
  page: CustomPage;
  start: PickerStart;
  library: Library | null;
  lookup: Lookup;
  macros: Macro[];
  onOpenMacros: () => void;
  onPlace: (pad: number, contents: CustomPad[]) => void;
  onClose: () => void;
}

export function Picker({ page, start, library, lookup, macros, onOpenMacros, onPlace, onClose }: PickerProps) {
  const texts = useTexts();
  const [tab, setTab] = useState<Tab>(start.tab);
  const [chosen, setChosen] = useState<string[]>(start.chosen);
  const [query, setQuery] = useState("");
  const [soundboard, setSoundboard] = useState("");
  const current = page.pads[start.pad];

  const choices = useMemo((): Choice[] => {
    const search = query.trim().toLowerCase();
    const matches = (name: string) => !search || name.toLowerCase().includes(search);
    switch (tab) {
      case "sounds": {
        const sounds: Choice[] = (library?.soundboards ?? [])
          .filter((board) => !soundboard || board.id === soundboard)
          .flatMap((board) =>
            board.sounds
              .filter((sound) => matches(sound.name))
              .map((sound) => ({
                key: `sound:${sound.id}`,
                id: `${board.id}:${sound.id}`,
                name: sound.name,
                sub: board.name,
                picture: sound.picture,
                locked: !sound.enabled,
              }))
          );
        // Across every soundboard, those in the licence first
        return soundboard ? sounds : [...sounds].sort((a, b) => Number(a.locked) - Number(b.locked));
      }
      case "voices":
        return [...(library?.voices ?? [])]
          .sort((a, b) => Number(b.enabled) - Number(a.enabled))
          .filter((voice) => matches(voice.name))
          .map((voice) => ({
            key: `voice:${voice.id}`,
            id: voice.id,
            name: voice.name,
            sub: texts.voiceKind,
            voice: true,
            picture: voice.picture,
            locked: !voice.enabled,
          }));
      case "actions":
        return ACTIONS.map((action) => ({
          key: `action:${action}`,
          id: action,
          name: texts.actionNames[action],
          sub: texts.actionTexts[action],
          action,
          locked: false,
        }));
      case "macros":
        return macros.map((item) => {
          const lengths = item.steps.map((step) => stepLength(step, library) ?? 0);
          const length = lengths.reduce((sum, next) => sum + next, 0);
          return {
            key: `macro:${item.id}`,
            id: item.id,
            name: item.name,
            sub: texts.macroSteps(item.steps.length, texts.secondsOf(length)),
            macro: true,
            tint: item.colour ?? MACRO_COLOUR,
            locked: false,
          };
        });
    }
  }, [tab, library, soundboard, query, texts, macros]);

  // As many as there are pads for: the clicked one and the free ones
  const room = start.replace ? 1 : padsFrom(page.pads, start.pad).length;
  const toggle = (key: string) =>
    setChosen((keys) => {
      if (keys.includes(key)) return keys.filter((other) => other !== key);
      if (start.replace) return [key];
      return keys.length < room ? [...keys, key] : keys;
    });

  const content = (key: string): CustomPad => {
    // Chosen again, the pad keeps its name, colour and page to open
    if (current && keyOf(current) === key) return current;
    const split = key.indexOf(":");
    const kind = key.slice(0, split);
    const id = key.slice(split + 1);
    if (kind === "sound") return { kind, id, name: lookup.sound(id)?.item.name ?? "", label: "", colour: null };
    if (kind === "voice") return { kind, id, name: lookup.voice(id)?.name ?? "", label: "", colour: null };
    if (kind === "macro") return { kind, id, label: "", colour: null };
    const action = id as Action;
    const target = action === "goToPage" ? lookup.targets.find((other) => other.id !== page.id)?.id : undefined;
    return { kind: "action", action, page: target, label: "", colour: null };
  };

  const count = chosen.length;
  let primary = texts.chooseSomething;
  if (count === 1) primary = current ? texts.replace : texts.putOnPad;
  if (count > 1) primary = texts.putOnPads(count);
  let hint = "";
  if (!start.replace && count > 0 && count >= room) hint = texts.pageFull;
  else if (count > 1) hint = texts.firstGoesTo(start.pad + 1);
  else if (tab === "sounds" && !start.replace) hint = texts.pickMany;
  else if (tab === "voices") hint = texts.licensedVoices;
  else if (tab === "actions") hint = texts.actionsHint;
  else if (tab === "macros") hint = texts.macrosHint;
  let currentName = "";
  if (current?.kind === "action") currentName = current.label || texts.actionNames[current.action];
  else if (current?.kind === "macro") currentName = current.label || lookup.macro(current.id)?.name || "";
  else if (current) currentName = current.label || current.name;
  const needsVoicemod = (tab === "sounds" || tab === "voices") && !lookup.loaded;

  return (
    <Dialog
      label={texts.chooseFor(start.pad + 1)}
      title={texts.chooseFor(start.pad + 1)}
      subtitle={`${page.name} · ${current ? texts.nowOnPad(currentName) : texts.emptyPad}`}
      onClose={onClose}
    >
      <div className="picker-tabs">
        {(["sounds", "voices", "actions", "macros"] as const).map((id) => (
          <button
            key={id}
            type="button"
            aria-pressed={tab === id}
            onClick={() => {
              setTab(id);
              setChosen([]);
              setQuery("");
            }}
          >
            {texts.pickerTabs[id]}
          </button>
        ))}
      </div>
      {tab === "sounds" && !needsVoicemod && (
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
              ...(library?.soundboards ?? []).map((board) => ({ value: board.id, label: board.name })),
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
        {needsVoicemod ? (
          <p className="muted nothing">{texts.voicemodNeeded}</p>
        ) : choices.length === 0 && tab !== "macros" ? (
          <p className="muted nothing">{texts.noResults}</p>
        ) : (
          <div className={tab === "sounds" || tab === "voices" ? "choices" : "choices wide"}>
            {choices.map((choice) => {
              const on = chosen.includes(choice.key);
              return (
                <button
                  key={choice.id}
                  type="button"
                  className={["choice", on ? "on" : "", choice.locked ? "locked" : ""].join(" ")}
                  aria-pressed={on}
                  disabled={choice.locked}
                  title={choice.locked ? texts.locked : choice.name}
                  onClick={() => toggle(choice.key)}
                >
                  <PadPicture
                    voice={choice.voice}
                    picture={choice.picture}
                    action={choice.action}
                    macro={choice.macro}
                    tint={choice.tint}
                  />
                  <span className="text">
                    <span className="name">{choice.name}</span>
                    <span className="muted">{choice.sub}</span>
                  </span>
                  {on && (
                    <span className="check">
                      <CheckIcon />
                    </span>
                  )}
                  {choice.locked && <LockIcon />}
                </button>
              );
            })}
            {tab === "macros" && (
              <button type="button" className="choice create" onClick={onOpenMacros}>
                <span className="picture">
                  <PlusIcon />
                </span>
                <span className="text">
                  <span className="name">{texts.createMacro}</span>
                  <span className="muted">{texts.createMacroText}</span>
                </span>
              </button>
            )}
          </div>
        )}
      </div>
      <footer>
        <p className="muted">{hint}</p>
        <button type="button" className="button secondary" onClick={onClose}>
          {texts.cancel}
        </button>
        <button type="button" className="button" disabled={count === 0} onClick={() => onPlace(start.pad, chosen.map(content))}>
          {primary}
        </button>
      </footer>
    </Dialog>
  );
}
