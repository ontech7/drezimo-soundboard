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
 * Plays `script` again and again while the window is in sight. `key` names everything the script reads: a new one
 * starts it over, as the window coming back does.
 */
export function useLoop(script: (run: Run) => Promise<void>, key: string): void {
  useEffect(() => {
    let run: ReturnType<typeof newRun> | null = null;
    const start = () => {
      run?.stop();
      run = null;
      if (document.hidden) return;
      const current = (run = newRun());
      (async () => {
        for (;;) await script(current);
      })();
    };
    start();
    document.addEventListener("visibilitychange", start);
    return () => {
      document.removeEventListener("visibilitychange", start);
      run?.stop();
    };
    // The key stands for the script: each render makes a new closure of the same one
  }, [key]);
}
