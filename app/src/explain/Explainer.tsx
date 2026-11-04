import { type ReactNode, useState } from "react";
import type { Library, PageTarget } from "../customPages";
import { useTexts } from "../i18n";
import { BRIDGE_ACTIONS, goneTarget } from "../knob";
import { type Macro, stepLength } from "../macros";
import { describe } from "../pages/MacrosPage";
import { type KindMode, type Outcome, voicemodsOwn } from "../pressAgain";
import type { PressAction, PressAgain, SettingsView } from "../settings";
import { ActionHalf } from "./ActionHalf";
import "./explain.css";
import { type Beats, KNOB_AT_REST, KnobHalf, type KnobPose, playGesture } from "./KnobHalf";
import { useLoop } from "./loop";
import { PAD_AT_REST, type PadCaption, PadHalf, type PadPose, playPad, type SoundCaption } from "./PadHalf";
import { type Lane, SoundLanes } from "./SoundLanes";
import { Half, Stage } from "./Stage";
import { type Behaviour, type KnobGesture, knobStory, overAtOnce, padStory, type Shown } from "./story";
import { type ExplainTexts, type GestureCaption, useExplainTexts } from "./texts";

/** The captions of a knob at rest, muted */
const STILL: GestureCaption[] = ["still", "released"];

/** Why a press does nothing with no macro or page chosen, or with a macro that has no steps */
function unsetTarget(press: PressAction, macro: Macro | undefined, words: ExplainTexts): string | undefined {
  if (press.action === "macro" && press.macroId === "") return words.idle.noMacro;
  if (press.action === "macro" && macro?.steps.length === 0) return words.idle.emptyMacro;
  if (press.action === "page" && press.page === "") return words.idle.noPage;
  return undefined;
}

interface KnobProps {
  gesture: KnobGesture;
  settings: SettingsView;
  bridge: boolean;
  macros: Macro[] | null;
  targets: PageTarget[];
  library: Library | null;
  /** The row's controls */
  children: ReactNode;
  onClose: () => void;
}

/** How a knob gesture works: the gesture, then what it does with the current settings */
export function KnobExplainer({ gesture, settings, bridge, macros, targets, library, children, onClose }: KnobProps) {
  const texts = useTexts();
  const words = useExplainTexts();
  const story = knobStory(gesture, settings.knob);
  const { press, borrowed } = story;
  const chosen = story.shown;
  const name = chosen.kind === "turn" ? texts.knobTab.turnActions[chosen.action] : texts.knobTab.pressActions[chosen.action];
  const chosenMacro = press?.action === "macro" ? macros?.find((macro) => macro.id === press.macroId) : undefined;
  // A target the board cannot find or run: the gesture does nothing, and the note says why
  const idle = press
    ? (goneTarget(press, macros, targets, library, texts) ?? unsetTarget(press, chosenMacro, words))
    : undefined;
  // Two quick taps on a macro over at once run it twice: the second one finds nothing to stop
  const twice =
    borrowed &&
    gesture === "doubleTap" &&
    !idle &&
    chosenMacro !== undefined &&
    overAtOnce(
      chosenMacro.repeat,
      chosenMacro.steps.map((step) => stepLength(step, library))
    );
  const macro = chosenMacro
    ? {
        name: chosenMacro.name,
        steps: chosenMacro.steps.slice(0, 4).map((step) => describe(step, texts, library, targets).title),
        again: twice ? ("restart" as const) : chosenMacro.again,
      }
    : null;
  const pageName = press?.action === "page" ? (targets.find((page) => page.id === press.page)?.name ?? null) : null;
  const shown: Shown = idle ? { kind: "press", action: "nothing" } : chosen;

  let sentence = chosen.kind === "turn" ? words.sentences.turn[chosen.action] : words.sentences.press[chosen.action];
  if (borrowed && gesture === "longPress") sentence = words.borrowed.longPress(name);
  // Two taps: the second one picks the menu's page, or stops or restarts the macro the first one started
  else if (borrowed && chosen.action === "menu") sentence = words.borrowed.menu;
  else if (borrowed && chosen.action === "macro" && !twice) sentence = words.borrowed.macro[macro?.again ?? "stop"];
  else if (borrowed) sentence = words.borrowed.doubleTap(name);
  const bridgeMissing = !bridge && BRIDGE_ACTIONS.has(chosen.action) ? texts.knobTab.bridgeMissing : undefined;

  const [pose, setPose] = useState<KnobPose>(KNOB_AT_REST);
  const [caption, setCaption] = useState<GestureCaption>("still");
  const [beats, setBeats] = useState<Beats>({ count: 0, sum: 0, total: 0 });
  const [loop, setLoop] = useState(0);
  const [fired, setFired] = useState(0);
  // The loop starts over when what it shows changes, the halves from rest at once
  const key = JSON.stringify([gesture, story, shown, macro?.again]);
  const [shownKey, setShownKey] = useState(key);
  if (shownKey !== key) {
    setShownKey(key);
    setLoop((count) => count + 1);
    setPose(KNOB_AT_REST);
    setCaption("still");
    setBeats({ count: 0, sum: 0, total: 0 });
  }

  useLoop(async (run, round) => {
    setLoop((count) => count + 1);
    setPose(KNOB_AT_REST);
    // A switch goes back and forth between loops, from where it starts for this action
    setBeats((beats) => ({ count: 0, sum: 0, total: round === 0 ? 0 : beats.total }));
    await playGesture(
      gesture,
      story,
      run,
      (changed, next) => {
        setPose((current) => ({ ...current, ...changed }));
        if (next) setCaption(next);
      },
      (dir) => {
        setFired((count) => count + 1);
        setBeats((beats) => ({ count: beats.count + 1, sum: beats.sum + dir, total: beats.total + 1 }));
      }
    );
  }, key);

  return (
    <Stage
      title={texts.knobTab.gestures[gesture]}
      subtitle={texts.knobTab.title}
      left={<KnobHalf key={`knob ${loop}`} pose={pose} caption={words.gestures[caption]} live={!STILL.includes(caption)} />}
      right={
        <ActionHalf
          key={`action ${loop}`}
          shown={shown}
          name={name}
          beats={beats}
          words={words}
          macro={macro}
          pageName={pageName}
          released={gesture === "holdTurn" && !pose.held}
        />
      }
      fired={fired}
      sentence={sentence}
      note={idle ?? bridgeMissing}
      choiceLabel={words.does}
      repeats={words.repeats}
      closeLabel={texts.close}
      onClose={onClose}
    >
      {children}
    </Stage>
  );
}

interface PadProps {
  mode: KindMode;
  choice: PressAgain;
  /** What the app makes of the choice for this type of sound, once known */
  outcome: Outcome | null;
  /** The row's control */
  children: ReactNode;
  onClose: () => void;
}

/** How a pad pressed again works for a type of sound: the pad, and what happens to its sound */
export function PadExplainer({ mode, choice, outcome, children, onClose }: PadProps) {
  const texts = useTexts();
  const words = useExplainTexts();
  // A type of sound always has a behaviour of its own in Voicemod
  const story = padStory(choice, voicemodsOwn(mode) as Behaviour, outcome);
  const [pose, setPose] = useState<PadPose>(PAD_AT_REST);
  const [caption, setCaption] = useState<PadCaption>("pad");
  const [sound, setSound] = useState<SoundCaption | null>(null);
  const [lanes, setLanes] = useState<Lane[]>([]);
  const [fired, setFired] = useState(0);
  const key = `${mode} ${JSON.stringify(story)}`;
  // Another choice starts from rest at once, never shown for a frame with the last one's sounds
  const [shownKey, setShownKey] = useState(key);
  if (shownKey !== key) {
    setShownKey(key);
    setPose(PAD_AT_REST);
    setCaption("pad");
    setSound(null);
    setLanes([]);
  }

  useLoop(
    (run) =>
      playPad(
        story,
        run,
        {
          pose: (changed, next) => {
            setPose((current) => ({ ...current, ...changed }));
            if (next) setCaption(next);
          },
          sound: setSound,
          lanes: (update) => setLanes((current) => update(current, performance.now())),
          fire: () => setFired((count) => count + 1),
        },
        words.pad,
        [words.pad.sounds[mode], words.sample.sounds[1]]
      ),
    key
  );

  const sentence = [
    story.asInVoicemod && words.pad.asInVoicemod,
    words.pad.sentences[story.behaviour],
    story.stopsAll && words.pad.stopsAll,
    story.instead && words.pad.instead,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <Stage
      title={texts.kindNames[mode]}
      subtitle={texts.padPressedAgain}
      left={<PadHalf pose={pose} caption={words.pad.captions[caption]} live={caption !== "pad"} />}
      right={
        <Half caption={sound ? words.pad.sound[sound] : words.pad.sounds[mode]} live={sound !== null}>
          <SoundLanes lanes={lanes} />
        </Half>
      }
      fired={fired}
      sentence={sentence}
      choiceLabel={texts.pressAgainField}
      repeats={words.repeats}
      closeLabel={texts.close}
      onClose={onClose}
    >
      {children}
    </Stage>
  );
}
