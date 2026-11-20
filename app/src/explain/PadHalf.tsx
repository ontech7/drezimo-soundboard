import type { CSSProperties } from "react";
import { PLAYING_COLOUR, rgb, SOUND_COLOUR } from "../customPages";
import type { Run } from "./loop";
import { held, type Lane } from "./SoundLanes";
import { Half } from "./Stage";
import type { PadStory } from "./story";
import type { ExplainTexts } from "./texts";

/** The pad as the left half draws it, lit as on the board */
export interface PadPose {
  led: "rest" | "playing" | "paused";
  down: boolean;
  /** Which press this is, as words.counts says it */
  count: string;
}

export const PAD_AT_REST: PadPose = { led: "rest", down: false, count: "" };

export type PadCaption = keyof ExplainTexts["pad"]["captions"];
export type SoundCaption = keyof ExplainTexts["pad"]["sound"];

/** What a pad script changes */
export interface PadStage {
  pose(changes: Partial<PadPose>, caption?: PadCaption): void;
  sound(caption: SoundCaption | null): void;
  lanes(update: (lanes: Lane[], now: number) => Lane[]): void;
  fire(): void;
}

export function PadHalf({ pose, caption, live }: { pose: PadPose; caption: string; live: boolean }) {
  const classes = ["cap lit explain-pad", pose.down && "pressed", pose.led === "paused" && "breathe"];
  const led = { "--led": rgb(pose.led === "rest" ? SOUND_COLOUR : PLAYING_COLOUR) } as CSSProperties;
  return (
    <Half caption={caption} live={live}>
      <span className={classes.filter(Boolean).join(" ")} style={led} />
      <span className={pose.count ? "explain-count on" : "explain-count"}>{pose.count}</span>
    </Half>
  );
}

// The sounds' lengths in ms: a song plays longer before its pause, a held one outlasts the hold
const SOUND = 3200;
const SONG = 3600;
const HELD = 4000;

// The lanes: another pad's sound, this pad's, and its copy
const OTHER = 0;
const OWN = 1;
const COPY = 2;

/** One loop of a pad pressed again, as its story says */
export async function playPad(story: PadStory, run: Run, stage: PadStage, words: ExplainTexts["pad"], names: [string, string]) {
  const [sound, other] = names;
  const change = (id: number, update: (lane: Lane, now: number) => Lane) =>
    stage.lanes((lanes, now) => lanes.map((lane) => (lane.id === id ? update(lane, now) : lane)));
  const press = async (caption: PadCaption, which: 0 | 1 | 2) => {
    stage.pose({ down: true, count: words.counts[which] }, caption);
    stage.fire();
    await run.wait(130);
    stage.pose({ down: false });
  };
  // When Voicemod cannot stop one sound, stopping or starting this one stops the other too
  const stopOthers = () => {
    if (story.stopsAll) change(OTHER, (lane, now) => held(lane, now, "stopped"));
  };
  const stopped = () => stage.sound(story.stopsAll ? "allStop" : "stops");

  const start = performance.now();
  const length = story.behaviour === "pause" ? SONG : story.behaviour === "hold" ? HELD : SOUND;
  const lanes: Lane[] = [{ id: OWN, name: sound, length, played: 0, since: start, state: "paused" }];
  if (story.stopsAll) {
    lanes.unshift({
      id: OTHER,
      name: other,
      note: words.otherNote,
      colour: "#7c8cff",
      length: 9000,
      played: 800,
      since: start,
      state: "playing",
    });
  }
  stage.pose(PAD_AT_REST, "pad");
  stage.sound(story.stopsAll ? "otherPlaying" : null);
  stage.lanes(() => lanes);
  await run.wait(story.stopsAll ? 1200 : 500);

  if (story.behaviour === "hold") {
    stage.pose({ down: true, led: "playing" }, "hold");
    stage.fire();
    change(OWN, (lane, now) => ({ ...lane, since: now, state: "playing" }));
    stage.sound("playing");
    await run.wait(1900);
    stage.pose({ down: false, led: "rest" }, "letGo");
    stage.fire();
    change(OWN, (lane, now) => held(lane, now, "stopped"));
    stopOthers();
    stage.sound(story.stopsAll ? "allStop" : "stopsOnRelease");
    await run.wait(1600);
    return;
  }

  await press("press", 0);
  change(OWN, (lane, now) => ({ ...lane, since: now, state: "playing" }));
  stage.pose({ led: "playing" });
  stage.sound("playing");

  switch (story.behaviour) {
    case "restart":
      await run.wait(1500);
      await press("again", 1);
      change(OWN, (lane, now) => ({ ...lane, played: 0, since: now, badge: words.restartBadge }));
      stopOthers();
      stage.sound("restarts");
      await run.wait(900);
      change(OWN, (lane) => ({ ...lane, badge: undefined }));
      await run.wait(SOUND - 900);
      break;
    case "overlap":
      await run.wait(1300);
      await press("again", 1);
      stage.lanes((current, now) => [
        ...current,
        { id: COPY, name: sound, note: words.copyNote, length: SOUND, played: 0, since: now, state: "playing" },
      ]);
      stopOthers();
      stage.sound("copies");
      await run.wait(SOUND);
      break;
    case "stop":
      await run.wait(1500);
      await press("again", 1);
      change(OWN, (lane, now) => held(lane, now, "stopped"));
      stopOthers();
      stage.pose({ led: "rest" });
      stopped();
      await run.wait(1500);
      break;
    case "pause":
      await run.wait(1300);
      await press("againShort", 1);
      change(OWN, (lane, now) => ({ ...held(lane, now, "paused"), badge: words.pauseBadge }));
      stopOthers();
      stage.pose({ led: "paused" });
      stage.sound("paused");
      await run.wait(1500);
      await press("againMore", 2);
      change(OWN, (lane, now) => ({ ...lane, since: now, state: "playing", badge: undefined }));
      stage.pose({ led: "playing" });
      stage.sound("resumes");
      await run.wait(SONG - 1300);
      break;
  }
  stage.pose({ led: "rest", count: "" });
  await run.wait(600);
}
