import { useEffect, useRef } from "react";

/** Closes a menu or popover on a click elsewhere or Escape. */
export function useDismiss<T extends HTMLElement>(open: boolean, close: () => void) {
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
