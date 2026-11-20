import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

export interface Run {
  wait(ms: number): Promise<void>;
}

/** A stopped run's waits never come back, so its script goes no further */
function newRun(): Run & { stop(): void } {
  let alive = true;
  const timers = new Set<number>();
  return {
    wait: (ms) =>
      new Promise((resolve) => {
        if (!alive) return;
        const timer = window.setTimeout(() => {
          timers.delete(timer);
          if (alive) resolve();
        }, ms);
        timers.add(timer);
      }),
    stop() {
      alive = false;
      timers.forEach((timer) => clearTimeout(timer));
    },
  };
}

/**
 * Plays `script` again and again while the window is in sight, `round` counting from 0. `key` names everything the
 * script reads: a new one starts it over, as the window coming back does.
 */
export function useLoop(script: (run: Run, round: number) => Promise<void>, key: string): void {
  useEffect(() => {
    let run: ReturnType<typeof newRun> | null = null;
    const stop = () => {
      run?.stop();
      run = null;
    };
    const start = () => {
      stop();
      if (document.hidden) return;
      const current = (run = newRun());
      (async () => {
        for (let round = 0; ; round++) await script(current, round);
      })();
    };
    // Hidden in the tray, WebView2 is not told: the loop stops with the window, and goes on once it has the focus again
    const resume = () => {
      if (!run) start();
    };
    start();
    document.addEventListener("visibilitychange", start);
    window.addEventListener("focus", resume);
    const unlisten = listen("window-closed", stop);
    return () => {
      document.removeEventListener("visibilitychange", start);
      window.removeEventListener("focus", resume);
      unlisten.then((off) => off()).catch(() => {});
      stop();
    };
    // The key stands for the script: each render makes a new closure of the same one
  }, [key]);
}
