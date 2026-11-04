import { type KeyboardEvent, type ReactNode, useEffect, useId, useRef, useState } from "react";
import { Row, Section } from "./controls";
import { useCustomPages, useLibrary } from "./customPages";
import { useDismiss } from "./dismiss";
import { useTexts } from "./i18n";
import { BoardIcon, CheckIcon, PressAgainIcon } from "./icons";
import {
  CHOICES,
  choicesFor,
  type KindMode,
  MODE_ROWS,
  type Outcome,
  type Outcomes,
  outcomeText,
  type PlaybackMode,
  reach,
  type Reach,
  soundNamesByMode,
  usePressOutcomes,
} from "./pressAgain";
import { SearchSelect } from "./SearchSelect";
import { type PressAgain, type PressAgainByMode, type SettingsView, saveSettings } from "./settings";

const MARKS: Record<Reach, string> = { chosen: "✓", stopsAll: "!", instead: "→" };
/** How many of a kind's sound names show under it */
const NAMES_SHOWN = 3;

/** The Drezimo page's choice for each kind of sound, and how many of the user's sounds are of that kind */
export function PadTab(props: { settings: SettingsView; singleStop: boolean; onOpenPages: () => void }) {
  const texts = useTexts();
  const library = useLibrary();
  const [pages] = useCustomPages();
  const outcomes = usePressOutcomes(props.singleStop);
  const byMode = props.settings.pressAgainByMode;
  // Before Voicemod first sends its soundboards there is nothing to count
  const sounds = library?.soundboards.length ? soundNamesByMode(library) : null;
  const ownChoices = (pages ?? []).flatMap((page) => page.pads).filter((pad) => pad?.kind === "sound" && pad.pressAgain).length;

  const choose = (key: keyof PressAgainByMode, choice: PressAgain) =>
    saveSettings({ ...props.settings, pressAgainByMode: { ...byMode, [key]: choice } });
  const label = (mode: KindMode, choice: PressAgain) => {
    if (choice === "voicemod") return `${texts.pressAgainNames.voicemod} · ${texts.voicemodDoes[mode]}`;
    const name = texts.pressAgainNames[choice];
    return outcomes?.[mode][choice].stopsAll ? `${name} · ${texts.alsoStopsOthers}` : name;
  };

  return (
    <Section title={texts.padPressedAgain} text={texts.pressAgainText}>
      {MODE_ROWS.map(([mode, key]) => {
        const names = sounds?.get(mode) ?? [];
        const choice = byMode[key];
        const options = (outcomes ? choicesFor(mode, outcomes) : [choice]).map((value) => ({
          value,
          label: label(mode, value),
        }));
        return (
          <Row
            key={mode}
            label={texts.kindNames[mode]}
            text={sounds ? texts.soundsOfKind(names.length, names.slice(0, NAMES_SHOWN)) : undefined}
            disabled={sounds !== null && names.length === 0}
          >
            <div className={choice === "voicemod" ? "press-select" : "press-select own"}>
              <SearchSelect
                label={texts.kindNames[mode]}
                options={options}
                value={choice}
                onChange={(picked) => choose(key, picked)}
                align="right"
              />
            </div>
          </Row>
        );
      })}
      <p className="muted press-own">
        {texts.ownChoices(ownChoices)} ·{" "}
        <button type="button" className="link" onClick={props.onOpenPages}>
          {texts.seeInPages}
        </button>
      </p>
    </Section>
  );
}

interface FieldProps {
  /** Picked for this pad, if any */
  own: PressAgain | null;
  /** As set on the Drezimo page for the sound's kind */
  typeChoice: PressAgain;
  /** The sound's own rule in Voicemod, once known */
  mode: PlaybackMode | null;
  outcomes: Outcomes | null;
  onChange: (choice: PressAgain | null) => void;
}

/** A sound pad's choice in the Pages editor, each option saying what it does to that sound */
export function PressAgainField({ own, typeChoice, mode, outcomes, onChange }: FieldProps) {
  const texts = useTexts();
  const labelId = useId();
  const valueId = useId();
  const headingId = useId();
  const [open, setOpen] = useState(false);
  const anchor = useDismiss<HTMLDivElement>(open, () => setOpen(false));
  const button = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const outcomeOf = (choice: PressAgain): Outcome | null => (mode && outcomes ? outcomes[mode][choice] : null);
  const choice = own ?? typeChoice;
  const outcome = outcomeOf(choice);

  useEffect(() => {
    if (open) menu.current?.querySelector<HTMLElement>('[aria-checked="true"]')?.focus();
  }, [open]);

  const pick = (picked: PressAgain | null) => {
    onChange(picked);
    setOpen(false);
    button.current?.focus();
  };
  const keys = (event: KeyboardEvent) => {
    // As the app's other menus: Escape and Tab close it, the focus back on the button
    if (event.key === "Escape" || event.key === "Tab") {
      event.preventDefault();
      setOpen(false);
      button.current?.focus();
      return;
    }
    const items = Array.from(menu.current?.querySelectorAll<HTMLElement>('[role="menuitemradio"]') ?? []);
    const at = items.indexOf(document.activeElement as HTMLElement);
    const next = { ArrowDown: at + 1, ArrowUp: at - 1, Home: 0, End: items.length - 1 }[event.key];
    if (next === undefined) return;
    event.preventDefault();
    items[(next + items.length) % items.length]?.focus();
  };
  const item = (option: PressAgain | null, name: ReactNode) => {
    const shown = option ?? typeChoice;
    const result = outcomeOf(shown);
    const kind = result && reach(shown, result);
    return (
      <button
        key={option ?? "typeChoice"}
        type="button"
        role="menuitemradio"
        aria-checked={option === own}
        className="press-item"
        onClick={() => pick(option)}
      >
        {option ? <PressAgainIcon choice={option} /> : <BoardIcon />}
        <span className="name">{name}</span>
        <CheckIcon />
        {result && kind && mode && (
          <span className={`outcome ${kind}`}>
            {MARKS[kind]} {outcomeText(texts, shown, mode, result)}
          </span>
        )}
      </button>
    );
  };

  return (
    <div className="field press-field">
      <span id={labelId}>{texts.pressAgainField}</span>
      <div className="anchor" ref={anchor}>
        <button
          ref={button}
          type="button"
          className="select-button"
          aria-haspopup="menu"
          aria-expanded={open}
          aria-labelledby={`${labelId} ${valueId}`}
          title={outcome && mode ? outcomeText(texts, choice, mode, outcome) : undefined}
          onClick={() => setOpen(!open)}
        >
          <PressAgainIcon choice={choice} />
          <span id={valueId}>
            {own ? texts.pressAgainNames[own] : `${texts.asItsKind} · ${texts.pressAgainNames[typeChoice]}`}
          </span>
          {outcome?.stopsAll && <span className="stops-all" />}
        </button>
        {open && (
          <div ref={menu} className="menu press-menu" role="menu" aria-labelledby={labelId} onKeyDown={keys}>
            {item(
              null,
              <>
                {texts.asItsKind} <span className="muted">· {texts.pressAgainNames[typeChoice]}</span>
              </>
            )}
            <hr />
            <div role="group" aria-labelledby={headingId}>
              <p id={headingId} className="muted heading">
                {texts.onlyThisPad}
              </p>
              {CHOICES.map((option) => item(option, texts.pressAgainNames[option]))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
