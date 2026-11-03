import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";

interface Store {
  /** Announced by the app whenever it changes */
  event: string;
  read: string;
  write: string;
  /** The write command's argument */
  argument: string;
}

/**
 * A list the app keeps in a file, as the window edits it: saved a moment after the last change. Also why the last
 * save failed, if it did, and a way to save what is waiting at once.
 */
export function useSaved<T>(store: Store): [T[] | null, (items: T[]) => void, string | null, () => Promise<void>] {
  const [items, setItems] = useState<T[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  // While a change is on its way, an older copy coming back must not undo what was typed since
  const saving = useRef(false);
  const latest = useRef(0);
  const timer = useRef<number | undefined>(undefined);
  // What waits for the timer, if anything
  const waiting = useRef<(() => Promise<void>) | null>(null);
  const { event, read, write, argument } = store;

  useEffect(() => {
    let updated = false;
    const unlisten = listen<T[]>(event, (received) => {
      updated = true;
      if (!saving.current) setItems(received.payload);
    });
    unlisten
      .then(() => invoke<T[]>(read))
      .then((current) => {
        if (!updated) setItems(current);
      })
      .catch((reason) => console.error(`cannot follow ${event}`, reason));
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, [event, read]);

  const save = useCallback(
    (next: T[]) => {
      setItems(next);
      saving.current = true;
      const change = ++latest.current;
      window.clearTimeout(timer.current);
      const send = () => {
        waiting.current = null;
        return invoke(write, { [argument]: next })
          .then(() => setError(null))
          .catch((reason) => setError(String(reason)))
          .finally(() => {
            // A newer change may already be on its way
            if (latest.current === change) saving.current = false;
          });
      };
      waiting.current = send;
      timer.current = window.setTimeout(send, 300);
    },
    [write, argument]
  );

  const flush = useCallback(() => {
    window.clearTimeout(timer.current);
    return waiting.current?.() ?? Promise.resolve();
  }, []);

  return [items, save, error, flush];
}
