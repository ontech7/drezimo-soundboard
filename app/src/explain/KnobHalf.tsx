import type { Run } from "./loop";
import { Half } from "./Stage";
import type { KnobGesture, KnobStory } from "./story";
import type { GestureCaption } from "./texts";

/** The knob as the left half draws it */
export interface KnobPose {
  angle: number;
  down: boolean;
  held: boolean;
  /** The long press's ring, filling */
  ring: boolean;
  /** Bumped to play the ripple again */
  ripple: number;
  /** The turn arrow's side, 0 while hidden */
  arrow: -1 | 0 | 1;
}

export const KNOB_AT_REST: KnobPose = { angle: 0, down: false, held: false, ring: false, ripple: 0, arrow: 0 };

/** What the gesture has done in this loop: how many times, the sum of its turns, and how many since the dialog opened */
export interface Beats {
  count: number;
  sum: number;
  total: number;
}

export type Show = (pose: Partial<KnobPose>, caption?: GestureCaption) => void;
/** A turn by one detent, or a press acting (0) */
export type Beat = (dir: -1 | 0 | 1) => void;

const TICKS = Array.from({ length: 24 }, (_, i) => {
  const angle = (i / 24) * Math.PI * 2;
  const outer = i % 2 ? 73 : 76;
  return {
    x1: 74 + 70 * Math.sin(angle),
    y1: 74 - 70 * Math.cos(angle),
    x2: 74 + outer * Math.sin(angle),
    y2: 74 - outer * Math.cos(angle),
  };
});

export function KnobHalf({ pose, caption, live }: { pose: KnobPose; caption: string; live: boolean }) {
  const knob = ["explain-knob", pose.down && "down", pose.held && "held"].filter(Boolean).join(" ");
  const arrow = ["explain-turn", pose.arrow !== 0 && "on", pose.arrow < 0 && "left"].filter(Boolean).join(" ");
  return (
    <Half caption={caption} live={live}>
      <div className="explain-knob-wrap">
        <svg className="explain-ticks" viewBox="0 0 148 148">
          {TICKS.map((tick, i) => (
            <line key={i} {...tick} />
          ))}
        </svg>
        <svg className="explain-ring" viewBox="0 0 140 140">
          <circle className={pose.ring ? "filling" : undefined} cx="70" cy="70" r="66" pathLength={100} />
        </svg>
        <span className={knob} style={{ rotate: `${pose.angle}deg` }} />
        {pose.ripple > 0 && <span key={pose.ripple} className="explain-ripple" />}
        <svg className={arrow} viewBox="0 0 172 172">
          <path d="M119 15.3A78 78 0 0 1 156.7 53" />
          <path d="M158.3 44.1L156.7 53l-7.8-4.5" />
        </svg>
      </div>
    </Half>
  );
}

/** One loop of the gesture, with the board's timings (src-tauri/src/knob.rs) */
export async function playGesture(gesture: KnobGesture, story: KnobStory, run: Run, show: Show, beat: Beat) {
  let angle = 0;
  let ripple = 0;
  const turns = async (dir: -1 | 1, count: number) => {
    show({ arrow: dir }, dir > 0 ? "turnRight" : "turnLeft");
    for (let i = 0; i < count; i++) {
      angle += dir * 30;
      show({ angle });
      beat(dir);
      await run.wait(560);
    }
    show({ arrow: 0 });
  };
  const press = async (ms: number) => {
    show({ down: true });
    await run.wait(ms);
    ripple += 1;
    show({ down: false, ripple });
  };
  // A tap acts once it stops waiting for a second one
  const act = async () => {
    if (story.waits) {
      show({}, "waits");
      await run.wait(300);
    }
    beat(0);
  };

  show({}, "still");
  switch (gesture) {
    case "turn":
      await run.wait(700);
      await turns(1, 3);
      await run.wait(500);
      await turns(-1, 2);
      show({}, "still");
      await run.wait(900);
      break;
    case "holdTurn":
      await run.wait(700);
      show({ down: true, held: true }, "holdIt");
      await run.wait(650);
      await turns(1, 3);
      show({}, "turnBack");
      await run.wait(250);
      await turns(-1, 2);
      show({ down: false, held: false }, "released");
      await run.wait(1000);
      break;
    case "tap":
      await run.wait(900);
      show({}, "tap");
      await press(140);
      await act();
      show({}, "tapped");
      await run.wait(2300);
      break;
    case "doubleTap":
      await run.wait(900);
      show({}, "tapFirst");
      await press(120);
      // With no double tap of its own, each tap acts at once
      if (story.borrowed) beat(0);
      await run.wait(160);
      show({}, story.borrowed ? "tapAgain" : "tapAgainSoon");
      await press(120);
      beat(0);
      await run.wait(2400);
      break;
    case "longPress":
      await run.wait(900);
      show({ down: true, ring: true }, "holdLong");
      await run.wait(600);
      show({}, "letGo");
      await run.wait(250);
      ripple += 1;
      show({ down: false, ring: false, ripple });
      await act();
      await run.wait(2300);
      break;
  }
}
