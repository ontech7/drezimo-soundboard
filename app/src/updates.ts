import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

/** How the app's own updates go (src-tauri/src/updates.rs). Times are Unix milliseconds. */
export type UpdateState =
  | { state: "local" }
  | { state: "waiting" }
  | { state: "checking" }
  | { state: "upToDate"; checkedAt: number }
  | { state: "downloading"; version: string }
  | { state: "ready"; version: string }
  | { state: "failed"; at: number };

/** What is new in the version just installed (src-tauri/src/news.rs) */
export interface News {
  version: string;
  notes: string | null;
}

export function useUpdateState(): UpdateState | null {
  const [state, setState] = useState<UpdateState | null>(null);

  useEffect(() => {
    let updated = false;
    const unlisten = listen<UpdateState>("update-state", (event) => {
      updated = true;
      setState(event.payload);
    });
    unlisten
      .then(() => invoke<UpdateState>("update_state"))
      .then((current) => {
        if (!updated) setState(current);
      })
      .catch((error) => console.error("cannot follow the updates", error));
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, []);

  return state;
}

export function useNews(): [News | null, () => void] {
  const [news, setNews] = useState<News | null>(null);

  useEffect(() => {
    invoke<News | null>("news")
      .then(setNews)
      .catch((error) => console.error("cannot read what is new", error));
  }, []);

  const dismiss = () => {
    setNews(null);
    invoke("dismiss_news").catch((error) => console.error("cannot dismiss what is new", error));
  };
  return [news, dismiss];
}
