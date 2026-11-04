import { type CSSProperties, memo, useEffect, useState } from "react";

/** A sound as a bar that fills while it plays */
export interface Lane {
  id: number;
  name: string;
  note?: string;
  /** The played part's colour */
  colour?: string;
  /** In ms */
  length: number;
  /** Played before `since`, in ms */
  played: number;
  since: number;
  state: "playing" | "paused" | "stopped";
  badge?: string;
}

export function position(lane: Lane, now: number): number {
  const moving = lane.state === "playing" ? Math.max(0, now - lane.since) : 0;
  return Math.min(lane.length, lane.played + moving);
}

/** The lane where it got to by `now`, in its new state */
export function held(lane: Lane, now: number, state: Lane["state"]): Lane {
  return { ...lane, played: position(lane, now), since: now, state };
}

const BARS = 40;

function heights(name: string): number[] {
  return Array.from({ length: BARS }, (_, i) => 30 + 60 * Math.abs(Math.sin(i * 1.7 + name.length) * Math.cos(i * 0.45)));
}

/** The same for a name at every frame, so drawn once */
const Bars = memo(function Bars({ name }: { name: string }) {
  return heights(name).map((height, i) => <i key={i} style={{ height: `${height}%` }} />);
});

export function SoundLanes({ lanes }: { lanes: Lane[] }) {
  const [, setFrame] = useState(0);
  const now = performance.now();
  const moving = lanes.some((lane) => lane.state === "playing" && position(lane, now) < lane.length);
  useEffect(() => {
    if (!moving) return;
    // With less motion asked for, the bars move a second at a time, as the seconds shown
    if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
      const timer = window.setInterval(() => setFrame((count) => count + 1), 1000);
      return () => clearInterval(timer);
    }
    let frame = requestAnimationFrame(function draw() {
      setFrame((count) => count + 1);
      frame = requestAnimationFrame(draw);
    });
    return () => cancelAnimationFrame(frame);
  }, [moving]);

  return (
    <div className="explain-lanes">
      {lanes.map((lane) => {
        const at = position(lane, now);
        const over = lane.state === "stopped" || at >= lane.length;
        const style = { "--lane": lane.colour ?? "var(--ok)", "--at": at / lane.length } as CSSProperties;
        return (
          <div key={lane.id} className={over ? "explain-lane over" : "explain-lane"}>
            <div className="explain-lane-head">
              <b>{lane.name}</b>
              {lane.note && <span>{lane.note}</span>}
              <em>0:{String(Math.floor(at / 1000)).padStart(2, "0")}</em>
            </div>
            <div className="explain-track" style={style}>
              <Bars name={lane.name} />
              <span className="explain-played">
                <Bars name={lane.name} />
              </span>
              <span className="explain-head" />
              <span className={lane.badge ? "explain-badge on" : "explain-badge"}>{lane.badge}</span>
            </div>
          </div>
        );
      })}
    </div>
  );
}
