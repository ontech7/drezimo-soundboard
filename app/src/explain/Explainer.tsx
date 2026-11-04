import { type ReactNode, useState } from "react";
import type { Library, PageTarget } from "../customPages";
import { type Texts, useTexts } from "../i18n";
import { BRIDGE_ACTIONS, goneTarget } from "../knob";
import { type Macro, stepLength } from "../macros";
import { describe } from "../pages/MacrosPage";
import { type KindMode, type Outcome, voicemodsOwn } from "../pressAgain";
import type { KnobSettings, PressAction, PressAgain, SettingsView } from "../settings";
import { ActionHalf } from "./ActionHalf";
import "./explain.css";
import { type Beats, KNOB_AT_REST, KnobHalf, type KnobPose, playGesture } from "./KnobHalf";
import { useLoop } from "./loop";
import { PAD_AT_REST, type PadCaption, PadHalf, type PadPose, playPad, type SoundCaption } from "./PadHalf";
import { type Lane, SoundLanes } from "./SoundLanes";
import { Arrow, Half, Stage } from "./Stage";
import {
  type Behaviour,
  type KnobGesture,
  type KnobStory,
  knobStory,
  overAtOnce,
  type PadStory,
  padStory,
  type Shown,
} from "./story";
import { type ExplainTexts, type GestureCaption, useExplainTexts } from "./texts";

/** The captions of a knob at rest, muted */
const STILL: GestureCaption[] = ["still", "released"];

type MacroShown = { name: string; steps: string[]; again: "stop" | "restart" } | null;

function actionName(shown: Shown, texts: Texts): string {
  return shown.kind === "turn" ? texts.knobTab.turnActions[shown.action] : texts.knobTab.pressActions[shown.action];
}

/** What a knob gesture's explanation says under the stage */
function knobSentence(
  gesture: KnobGesture,
  story: KnobStory,
  name: string,
  macro: MacroShown,
  twice: boolean,
  words: ExplainTexts
) {
  const { shown, borrowed } = story;
  if (borrowed && gesture === "longPress") return words.borrowed.longPress(name);
  // Two taps: the second one picks the menu's page, or stops or restarts the macro the first one started
  if (borrowed && shown.action === "menu") return words.borrowed.menu;
  if (borrowed && shown.action === "macro" && !twice) return words.borrowed.macro[macro?.again ?? "stop"];
  if (borrowed) return words.borrowed.doubleTap(name);
  return shown.kind === "turn" ? words.sentences.turn[shown.action] : words.sentences.press[shown.action];
}

function padSentence(story: PadStory, words: ExplainTexts): string {
  return [
    story.asInVoicemod && words.pad.asInVoicemod,
    words.pad.sentences[story.behaviour],
    story.stopsAll && words.pad.stopsAll,
    story.instead && words.pad.instead,
  ]
    .filter(Boolean)
    .join(" ");
}

/** Why a press does nothing with no macro or page chosen, or with a macro that has no steps */
function unsetTarget(press: PressAction, macro: Macro | undefined, words: ExplainTexts): string | undefined {
  if (press.action === "macro" && press.macroId === "") return words.idle.noMacro;
  if (press.action === "macro" && macro?.steps.length === 0) return words.idle.emptyMacro;
  if (press.action === "page" && press.page === "") return words.idle.noPage;
  return undefined;
}

interface KnobSceneProps {
  gesture: KnobGesture;
  story: KnobStory;
  shown: Shown;
  name: string;
  words: ExplainTexts;
  macro: MacroShown;
  pageName: string | null;
}

/** The knob making the gesture, then what it does; given a new key for each new choice, so that it starts from rest */
function KnobScene({ gesture, story, shown, name, words, macro, pageName }: KnobSceneProps) {
  const [pose, setPose] = useState<KnobPose>(KNOB_AT_REST);
  const [caption, setCaption] = useState<GestureCaption>("still");
  const [beats, setBeats] = useState<Beats>({ count: 0, sum: 0, total: 0 });
  const [loop, setLoop] = useState(0);
  const [fired, setFired] = useState(0);

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
  }, gesture);

  return (
    <>
      <KnobHalf key={`knob ${loop}`} pose={pose} caption={words.gestures[caption]} live={!STILL.includes(caption)} />
      <Arrow fired={fired} />
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
    </>
  );
}

/** The pad pressed again and again, then what happens to its sound; a new key for each new choice */
function PadScene({ mode, story, words }: { mode: KindMode; story: PadStory; words: ExplainTexts }) {
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
    mode
  );

  return (
    <>
      <PadHalf pose={pose} caption={words.pad.captions[caption]} live={caption !== "pad"} />
      <Arrow fired={fired} />
      <Half caption={sound ? words.pad.sound[sound] : words.pad.sounds[mode]} live={sound !== null}>
        <SoundLanes lanes={lanes} />
      </Half>
    </>
  );
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
  const name = actionName(story.shown, texts);
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
  const shown: Shown = idle ? { kind: "press", action: "nothing" } : story.shown;
  const bridgeMissing = !bridge && BRIDGE_ACTIONS.has(story.shown.action) ? texts.knobTab.bridgeMissing : undefined;

  return (
    <Stage
      title={texts.knobTab.gestures[gesture]}
      subtitle={texts.knobTab.title}
      scene={
        <KnobScene
          key={JSON.stringify([gesture, story, shown, macro?.again])}
          gesture={gesture}
          story={story}
          shown={shown}
          name={name}
          words={words}
          macro={macro}
          pageName={pageName}
        />
      }
      sentence={knobSentence(gesture, story, name, macro, twice, words)}
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
  return (
    <Stage
      title={texts.kindNames[mode]}
      subtitle={texts.padPressedAgain}
      scene={<PadScene key={`${mode} ${JSON.stringify(story)}`} mode={mode} story={story} words={words} />}
      sentence={padSentence(story, words)}
      choiceLabel={texts.pressAgainField}
      repeats={words.repeats}
      closeLabel={texts.close}
      onClose={onClose}
    >
      {children}
    </Stage>
  );
}

/** For the welcome tour: a stage and its sentence, with sample names */
function Demo({ scene, sentence }: { scene: ReactNode; sentence: string }) {
  return (
    <div className="explain-demo">
      <div className="explain-stage" aria-hidden="true">
        {scene}
      </div>
      <p className="explain-sentence">{sentence}</p>
    </div>
  );
}

/** A knob gesture in the welcome tour, with the knob as the tour sets it */
export function KnobDemo({ gesture, knob }: { gesture: KnobGesture; knob: KnobSettings }) {
  const texts = useTexts();
  const words = useExplainTexts();
  const story = knobStory(gesture, knob);
  const name = actionName(story.shown, texts);
  return (
    <Demo
      scene={
        <KnobScene
          key={JSON.stringify([gesture, story])}
          gesture={gesture}
          story={story}
          shown={story.shown}
          name={name}
          words={words}
          macro={null}
          pageName={null}
        />
      }
      sentence={knobSentence(gesture, story, name, null, false, words)}
    />
  );
}

/** A pad pressed again in the welcome tour, as the choice would make it behave */
export function PadDemo({ mode, choice }: { mode: KindMode; choice: PressAgain }) {
  const words = useExplainTexts();
  const story = padStory(choice, voicemodsOwn(mode) as Behaviour, null);
  return (
    <Demo
      scene={<PadScene key={`${mode} ${JSON.stringify(story)}`} mode={mode} story={story} words={words} />}
      sentence={padSentence(story, words)}
    />
  );
}
