import { type KeyboardEvent, type ReactNode, useEffect, useId, useRef, useState } from "react";
import { Section } from "./controls";
import { useLibrary } from "./customPages";
import { useDismiss } from "./dismiss";
import { useTexts } from "./i18n";
import { BoardIcon, CheckIcon, ChevronDown, PressAgainIcon } from "./icons";
import {
  CHOICES,
  type Outcome,
  type Outcomes,
  outcomeText,
  type PlaybackMode,
  reach,
  type Reach,
  soundsByMode,
  usePressOutcomes,
} from "./pressAgain";
import { type PressAgain, type SettingsView, saveSettings } from "./settings";

const MARKS: Record<Reach, string> = { chosen: "✓", stopsAll: "!", instead: "→" };

/** The Drezimo page's choice for every pad, and what it does to the user's sounds */
export function PressAgainSection({ settings, singleStop }: { settings: SettingsView; singleStop: boolean }) {
  const texts = useTexts();
  const library = useLibrary();
  const outcomes = usePressOutcomes(singleStop);
  const choice = settings.pressAgain;
  const groups = soundsByMode(library);

  return (
    <Section title={texts.pads} text={texts.pressAgainText}>
      <div className="press-choices" role="radiogroup" aria-label={texts.pads}>
        {CHOICES.map((option) => (
          <button
            key={option}
            type="button"
            role="radio"
            aria-checked={option === choice}
            className="press-choice"
            onClick={() => saveSettings({ ...settings, pressAgain: option })}
          >
            <span className="glyph">
              <PressAgainIcon choice={option} />
            </span>
            <span className="name">
              {texts.pressAgainNames[option]}
              {option === "voicemod" && <span className="tag">{texts.byDefault}</span>}
            </span>
            <span className="muted">{texts.pressAgainTexts[option]}</span>
          </button>
        ))}
      </div>
      {outcomes && groups.length > 0 && <Effect choice={choice} groups={groups} outcomes={outcomes} />}
    </Section>
  );
}

function Effect(props: { choice: PressAgain; groups: [PlaybackMode, number][]; outcomes: Outcomes }) {
  const texts = useTexts();
  const [open, setOpen] = useState(false);
  const rows = props.groups.map(([mode, count]) => {
    const outcome = props.outcomes[mode][props.choice];
    return { mode, count, reach: reach(props.choice, outcome), text: outcomeText(texts, props.choice, mode, outcome) };
  });
  const total = (kind?: Reach) => rows.filter((row) => !kind || row.reach === kind).reduce((sum, row) => sum + row.count, 0);
  const counts: [Reach, string][] = [
    ["chosen", texts.chosenCount(total("chosen"))],
    ["stopsAll", texts.stopsAllCount(total("stopsAll"))],
    ["instead", texts.insteadCount(total("instead"))],
  ];

  return (
    <div className="press-effect">
      <div className="press-summary">
        <span className="muted">{texts.onYourSounds(total())}</span>
        {counts
          .filter(([kind]) => total(kind) > 0)
          .map(([kind, text]) => (
            <span key={kind} className={`press-count ${kind}`}>
              {MARKS[kind]} {text}
            </span>
          ))}
        <button type="button" className="disclosure" aria-expanded={open} onClick={() => setOpen(!open)}>
          {texts.soundBySound}
          <ChevronDown />
        </button>
      </div>
      {open && (
        <table className="press-details">
          <tbody>
            {rows.map((row) => (
              <tr key={row.mode}>
                <td className="muted">{texts.soundsWithMode(row.mode, row.count)}</td>
                <td className={row.reach}>
                  <span className="mark">{MARKS[row.reach]}</span>
                  {row.text}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

interface FieldProps {
  /** Picked for this pad, if any */
  own: PressAgain | null;
  /** As set on the Drezimo page */
  everyPad: PressAgain;
  /** The sound's own rule in Voicemod, once known */
  mode: PlaybackMode | null;
  outcomes: Outcomes | null;
  onChange: (choice: PressAgain | null) => void;
}

/** A sound pad's choice in the Pages editor, each option saying what it does to that sound */
export function PressAgainField({ own, everyPad, mode, outcomes, onChange }: FieldProps) {
  const texts = useTexts();
  const labelId = useId();
  const valueId = useId();
  const headingId = useId();
  const [open, setOpen] = useState(false);
  const anchor = useDismiss<HTMLDivElement>(open, () => setOpen(false));
  const button = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const outcomeOf = (choice: PressAgain): Outcome | null => (mode && outcomes ? outcomes[mode][choice] : null);
  const choice = own ?? everyPad;
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
    const shown = option ?? everyPad;
    const result = outcomeOf(shown);
    const kind = result && reach(shown, result);
    return (
      <button
        key={option ?? "everyPad"}
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
            {own ? texts.pressAgainNames[own] : `${texts.asForEveryPad} · ${texts.pressAgainNames[everyPad]}`}
          </span>
          {outcome?.stopsAll && <span className="stops-all" />}
        </button>
        {open && (
          <div ref={menu} className="menu press-menu" role="menu" aria-labelledby={labelId} onKeyDown={keys}>
            {item(
              null,
              <>
                {texts.asForEveryPad} <span className="muted">· {texts.pressAgainNames[everyPad]}</span>
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
