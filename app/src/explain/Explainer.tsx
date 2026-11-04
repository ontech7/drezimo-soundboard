import { type ReactNode, useState } from "react";
import type { Library, PageTarget } from "../customPages";
import { useTexts } from "../i18n";
import { BRIDGE_ACTIONS, goneTarget } from "../KnobControls";
import type { Macro } from "../macros";
import { describe } from "../pages/MacrosPage";
import { type KindMode, type Outcome, voicemodsOwn } from "../pressAgain";
import type { PressAction, PressAgain, SettingsView, TurnAction } from "../settings";
import { ActionHalf, type Shown } from "./ActionHalf";
import "./explain.css";
import { type Beats, KNOB_AT_REST, KnobHalf, type KnobPose, playGesture } from "./KnobHalf";
import { useLoop } from "./loop";
import { PAD_AT_REST, type PadCaption, PadHalf, type PadPose, playPad, type SoundCaption } from "./PadHalf";
import { type Lane, SoundLanes } from "./SoundLanes";
import { Half, Stage } from "./Stage";
import { type Behaviour, type KnobGesture, knobStory, padStory } from "./story";
import { explainTexts, type GestureCaption } from "./texts";

/** The captions of a knob at rest, muted */
const STILL: GestureCaption[] = ["still", "released"];

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
  const words = explainTexts(settings.shownLanguage);
  const story = knobStory(gesture, settings.knob);
  const turn = gesture === "turn" || gesture === "holdTurn";
  // A turn gesture's story always holds a turn action, a press gesture's a press action
  const shown: Shown = turn
    ? { kind: "turn", action: story.action as TurnAction["action"] }
    : { kind: "press", action: story.action as PressAction["action"] };
  const name = shown.kind === "turn" ? texts.knobTab.turnActions[shown.action] : texts.knobTab.pressActions[shown.action];
  // The macro or page of the press it shows: the tap's, when the gesture has no action of its own
  const press = story.borrowed ? settings.knob.tap : turn ? null : settings.knob[gesture as "tap" | "doubleTap" | "longPress"];
  const chosenMacro = press?.action === "macro" ? macros?.find((macro) => macro.id === press.macroId) : undefined;
  const macro = chosenMacro
    ? {
        name: chosenMacro.name,
        steps: chosenMacro.steps.slice(0, 4).map((step) => describe(step, texts, library, targets).title),
        again: chosenMacro.again,
      }
    : null;
  const pageName = press?.action === "page" ? (targets.find((page) => page.id === press.page)?.name ?? null) : null;
  let sentence = shown.kind === "turn" ? words.sentences.turn[shown.action] : words.sentences.press[shown.action];
  if (story.borrowed && gesture === "longPress") sentence = words.borrowed.longPress(name);
  // Two taps: the second one picks the menu's page, or stops or restarts the macro the first one started
  else if (story.borrowed && shown.action === "menu") sentence = words.borrowed.menu;
  else if (story.borrowed && shown.action === "macro") sentence = words.borrowed.macro[macro?.again ?? "stop"];
  else if (story.borrowed) sentence = words.borrowed.doubleTap(name);
  const gone = press ? goneTarget(press, macros, targets, library, texts) : undefined;
  const bridgeMissing = !bridge && BRIDGE_ACTIONS.has(story.action) ? texts.knobTab.bridgeMissing : undefined;
  const [pose, setPose] = useState<KnobPose>(KNOB_AT_REST);
  const [caption, setCaption] = useState<GestureCaption>("still");
  const [beats, setBeats] = useState<Beats>({ count: 0, sum: 0, total: 0 });
  const [loop, setLoop] = useState(0);
  const [fired, setFired] = useState(0);

  useLoop(
    async (run, round) => {
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
    },
    `${gesture} ${JSON.stringify(story)}`
  );

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
      note={gone ?? bridgeMissing}
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
  language: "english" | "italian";
  /** The row's control */
  children: ReactNode;
  onClose: () => void;
}

/** How a pad pressed again works for a type of sound: the pad, and what happens to its sound */
export function PadExplainer({ mode, choice, outcome, language, children, onClose }: PadProps) {
  const texts = useTexts();
  const words = explainTexts(language);
  // A type of sound always has a behaviour of its own in Voicemod
  const story = padStory(choice, voicemodsOwn(mode) as Behaviour, outcome);
  const [pose, setPose] = useState<PadPose>(PAD_AT_REST);
  const [caption, setCaption] = useState<PadCaption>("pad");
  const [sound, setSound] = useState<SoundCaption | null>(null);
  const [lanes, setLanes] = useState<Lane[]>([]);
  const [fired, setFired] = useState(0);

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
    `${mode} ${JSON.stringify(story)}`
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
