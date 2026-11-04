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
  /** Without it the menu has no search field, for a short list */
  searchLabel?: string;
  /** When nothing matches the search */
  empty?: string;
  /** On the button, while the value is none of the options */
  placeholder?: string;
  /** The menu's side, for a list at the right edge of a dialog */
  align?: "left" | "right";
}

/** The app's select, the same on Windows and macOS: for a long list its menu has a search field. */
export function SearchSelect<T extends string | number>(props: Props<T>) {
  const { options, value, onChange } = props;
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const button = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLUListElement>(null);
  // The list follows the keyboard, not the pointer: an item half in sight would scroll under it
  const follow = useRef(true);
  const id = useId();
  const root = useDismiss<HTMLDivElement>(open, () => setOpen(false));
  const searchable = props.searchLabel !== undefined;

  const shown = options.filter((option) => matches(query, option.label));
  const current = options.find((option) => option.value === value);

  const startingWith = (typed: string) => options.findIndex((option) => matches(typed, option.label.slice(0, typed.length)));
  const show = (typed: string) => {
    const start = !typed ? options.findIndex((option) => option.value === value) : searchable ? 0 : startingWith(typed);
    setQuery(searchable ? typed : "");
    setActive(Math.max(0, start));
    follow.current = true;
    setOpen(true);
  };
  const move = (index: number) => {
    follow.current = true;
    setActive(Math.min(Math.max(index, 0), shown.length - 1));
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
    if (open && follow.current) {
      list.current?.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: "nearest" });
    }
  }, [open, active]);

  useEffect(() => {
    if (!open) setQuery("");
    else if (!searchable) list.current?.focus();
  }, [open, searchable]);

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
      move(active + (event.key === "ArrowDown" ? 1 : -1));
    } else if (event.key === "Home" || event.key === "End") {
      event.preventDefault();
      move(event.key === "Home" ? 0 : shown.length - 1);
    } else if (event.key === "Tab") {
      // Leaving the field leaves the menu: the focus goes back to the button, the next Tab moves on
      event.preventDefault();
      close();
    } else if (event.key === "Enter") {
      event.preventDefault();
      const option = shown[active];
      if (option) pick(option.value);
    } else if (event.key === "Escape") {
      // Not the dialog around it, if any: only the menu closes
      event.preventDefault();
      event.stopPropagation();
      close();
    } else if (!searchable && event.key.length === 1 && event.key !== " ") {
      const found = startingWith(event.key);
      if (found >= 0) move(found);
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
        aria-controls={open ? id : undefined}
        onClick={() => (open ? close() : show(""))}
        onKeyDown={onButtonKey}
      >
        <span>{current?.label ?? props.placeholder ?? ""}</span>
      </button>
      {open && (
        <div className={["search-menu", props.align === "right" && "right", !searchable && "plain"].filter(Boolean).join(" ")}>
          {searchable && (
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
                  follow.current = true;
                  setActive(0);
                }}
                onKeyDown={onSearchKey}
              />
            </label>
          )}
          <ul
            ref={list}
            id={id}
            role="listbox"
            aria-label={props.label}
            tabIndex={searchable ? undefined : -1}
            aria-activedescendant={!searchable && shown[active] ? `${id}-${active}` : undefined}
            onKeyDown={searchable ? undefined : onSearchKey}
          >
            {shown.map((option, index) => (
              <li
                key={option.value}
                id={`${id}-${index}`}
                data-index={index}
                role="option"
                aria-selected={option.value === value}
                className={index === active ? "active" : undefined}
                onPointerMove={() => {
                  follow.current = false;
                  setActive(index);
                }}
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
