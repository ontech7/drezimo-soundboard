import { type CSSProperties, type ReactNode, useEffect, useRef, useState } from "react";
import type { Anchor } from "./settings";

export function Section({ title, text, children }: { title?: string; text?: string; children: ReactNode }) {
  return (
    <section className="card">
      {title && <h2>{title}</h2>}
      {text && <p className="muted section-text">{text}</p>}
      <div className="rows">{children}</div>
    </section>
  );
}

export function Row(props: { label: string; text?: string; disabled?: boolean; children?: ReactNode }) {
  return (
    <div className={props.disabled ? "row disabled" : "row"}>
      <div className="row-label">
        <p>{props.label}</p>
        {props.text && <p className="muted">{props.text}</p>}
      </div>
      {props.children && <div className="row-control">{props.children}</div>}
    </div>
  );
}

export function Toggle(props: { label: string; checked: boolean; disabled?: boolean; onChange: (checked: boolean) => void }) {
  return (
    <button
      type="button"
      role="switch"
      aria-label={props.label}
      aria-checked={props.checked}
      className={props.checked ? "toggle on" : "toggle"}
      disabled={props.disabled}
      onClick={() => props.onChange(!props.checked)}
    >
      <span />
    </button>
  );
}

interface SliderProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  disabled?: boolean;
  /** While it moves */
  onPreview?: (value: number) => void;
  /** Once it is let go */
  onCommit: (value: number) => void;
}

export function Slider({ label, value, min, max, step, disabled, onPreview, onCommit }: SliderProps) {
  const [shown, setShown] = useState(value);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => setShown(value), [value]);
  // The native change event comes once the slider is let go, wherever the pointer is; React's onChange comes at each move
  useEffect(() => {
    const element = input.current;
    const commit = () => {
      const next = Number(element?.value);
      if (next !== value) onCommit(next);
    };
    element?.addEventListener("change", commit);
    return () => element?.removeEventListener("change", commit);
  }, [value, onCommit]);
  const fill = { "--fill": `${((shown - min) / (max - min)) * 100}%` } as CSSProperties;

  return (
    <div className="slider">
      <input
        ref={input}
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={shown}
        disabled={disabled}
        style={fill}
        onChange={(event) => {
          const next = Number(event.target.value);
          setShown(next);
          onPreview?.(next);
        }}
      />
      <output>{shown}%</output>
    </div>
  );
}

export interface Option<T> {
  value: T;
  label: string;
}

export function Segmented<T extends string | number>(props: {
  label: string;
  options: Option<T>[];
  value: T;
  disabled?: boolean;
  onChange: (value: T) => void;
}) {
  return (
    <div className="segmented" role="radiogroup" aria-label={props.label}>
      {props.options.map((option) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={option.value === props.value}
          disabled={props.disabled}
          onClick={() => props.onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

/** Minutes and the like: a value set elsewhere, not among the options, still shows */
export function Choice(props: {
  label: string;
  options: Option<number>[];
  value: number;
  describe: (value: number) => string;
  disabled?: boolean;
  onChange: (value: number) => void;
}) {
  const known = props.options.some((option) => option.value === props.value);
  const options = known ? props.options : [...props.options, { value: props.value, label: props.describe(props.value) }];
  return (
    <select
      aria-label={props.label}
      value={props.value}
      disabled={props.disabled}
      onChange={(event) => props.onChange(Number(event.target.value))}
    >
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
  );
}

const ANCHORS: Anchor[] = ["topLeft", "top", "topRight", "left", "center", "right", "bottomLeft", "bottom", "bottomRight"];

/** A small screen with nine places for the overlay */
export function AnchorPicker(props: {
  label: string;
  names: Record<Anchor, string>;
  value: Anchor;
  disabled?: boolean;
  onChange: (anchor: Anchor) => void;
}) {
  return (
    <div className="screen" role="radiogroup" aria-label={props.label}>
      {ANCHORS.map((anchor) => (
        <button
          key={anchor}
          type="button"
          role="radio"
          aria-label={props.names[anchor]}
          aria-checked={anchor === props.value}
          disabled={props.disabled}
          onClick={() => props.onChange(anchor)}
        >
          <span />
        </button>
      ))}
      <div className="taskbar" />
    </div>
  );
}
