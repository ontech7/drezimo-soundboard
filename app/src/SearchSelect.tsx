import { type KeyboardEvent, useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { useDismiss } from "./dismiss";
import { CheckIcon, SearchIcon } from "./icons";

/** Whether `text` holds `query`, ignoring case and accents: "citta" finds "Città". */
export function matches(query: string, text: string): boolean {
  const plain = (value: string) => value.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
  return plain(text).includes(plain(query.trim()));
}

interface Option<T> {
  value: T;
  label: string;
}

interface Props<T> {
  label: string;
  options: Option<T>[];
  value: T;
  onChange: (value: T) => void;
  searchLabel: string;
  /** When nothing matches the search */
  empty: string;
  /** On the button, while the value is none of the options */
  placeholder?: string;
  /** The menu's side, for a list at the right edge of a dialog */
  align?: "left" | "right";
}

/** A select for long lists: its menu has a search field, the same on Windows and macOS. */
export function SearchSelect<T extends string | number>(props: Props<T>) {
  const { options, value, onChange } = props;
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const button = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const id = useId();
  const root = useDismiss<HTMLDivElement>(open, () => setOpen(false));

  const shown = options.filter((option) => matches(query, option.label));
  const current = options.find((option) => option.value === value);

  const show = (typed: string) => {
    const start = typed ? 0 : options.findIndex((option) => option.value === value);
    setQuery(typed);
    setActive(Math.max(0, start));
    setOpen(true);
  };
  const close = () => {
    setOpen(false);
    button.current?.focus();
  };
  const pick = (picked: T) => {
    close();
    if (picked !== value) onChange(picked);
  };

  // The current value, or the one the arrows reach, in sight
  useLayoutEffect(() => {
    if (open) list.current?.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: "nearest" });
  }, [open, active]);

  useEffect(() => {
    if (!open) setQuery("");
  }, [open]);

  // In a dialog, Escape closes the menu only: WebKit may cancel the dialog despite the key being handled
  useEffect(() => {
    const dialog = root.current?.closest("dialog");
    if (!open || !dialog) return;
    const cancel = (event: Event) => {
      event.preventDefault();
      setOpen(false);
      button.current?.focus();
    };
    dialog.addEventListener("cancel", cancel);
    return () => dialog.removeEventListener("cancel", cancel);
  }, [open, root]);

  const onButtonKey = (event: KeyboardEvent) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      show("");
    } else if (event.key.length === 1 && event.key !== " " && !event.metaKey && !event.ctrlKey && !event.altKey) {
      event.preventDefault();
      show(event.key);
    }
  };

  const onSearchKey = (event: KeyboardEvent) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      setActive((index) => Math.min(Math.max(index + step, 0), shown.length - 1));
    } else if (event.key === "Enter") {
      event.preventDefault();
      const option = shown[active];
      if (option) pick(option.value);
    } else if (event.key === "Escape") {
      // Not the dialog around it, if any: only the menu closes
      event.preventDefault();
      event.stopPropagation();
      close();
    }
  };

  return (
    <div className="search-select" ref={root}>
      <button
        ref={button}
        type="button"
        className="select-button"
        role="combobox"
        aria-label={props.label}
        aria-expanded={open}
        aria-haspopup="listbox"
        aria-controls={id}
        onClick={() => (open ? close() : show(""))}
        onKeyDown={onButtonKey}
      >
        <span>{current?.label ?? props.placeholder ?? ""}</span>
      </button>
      {open && (
        <div className={props.align === "right" ? "search-menu right" : "search-menu"}>
          <label className="search">
            <SearchIcon />
            <input
              type="search"
              aria-label={props.searchLabel}
              placeholder={props.searchLabel}
              aria-controls={id}
              aria-activedescendant={shown[active] ? `${id}-${active}` : undefined}
              value={query}
              autoFocus
              onChange={(event) => {
                setQuery(event.target.value);
                setActive(0);
              }}
              onKeyDown={onSearchKey}
            />
          </label>
          <ul ref={list} id={id} role="listbox" aria-label={props.label}>
            {shown.map((option, index) => (
              <li
                key={option.value}
                id={`${id}-${index}`}
                data-index={index}
                role="option"
                aria-selected={option.value === value}
                className={index === active ? "active" : undefined}
                onPointerMove={() => setActive(index)}
                onClick={() => pick(option.value)}
              >
                <span>{option.label}</span>
                {option.value === value && <CheckIcon />}
              </li>
            ))}
          </ul>
          {shown.length === 0 && <p className="muted">{props.empty}</p>}
        </div>
      )}
    </div>
  );
}
