import { type ReactNode, useState } from "react";
import type { Library, PageTarget } from "../customPages";
import { useTexts } from "../i18n";
import type { Macro } from "../macros";
import type { SettingsView } from "../settings";
import "./explain.css";
import { type Beats, KNOB_AT_REST, KnobHalf, type KnobPose, playGesture } from "./KnobHalf";
import { useLoop } from "./loop";
import { Half, Stage } from "./Stage";
import { type KnobGesture, knobStory } from "./story";
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
export function KnobExplainer({ gesture, settings, children, onClose }: KnobProps) {
  const texts = useTexts();
  const words = explainTexts(settings.shownLanguage);
  const story = knobStory(gesture, settings.knob);
  const [pose, setPose] = useState<KnobPose>(KNOB_AT_REST);
  const [caption, setCaption] = useState<GestureCaption>("still");
  const [, setBeats] = useState<Beats>({ count: 0, sum: 0, total: 0 });
  const [loop, setLoop] = useState(0);
  const [fired, setFired] = useState(0);

  useLoop(
    async (run) => {
      setLoop((count) => count + 1);
      setPose(KNOB_AT_REST);
      setBeats((beats) => ({ count: 0, sum: 0, total: beats.total }));
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
        <Half caption={texts.knobTab.pressActions.nothing} live={false}>
          <p className="explain-nothing">
            <b>—</b>
            {words.nothing}
          </p>
        </Half>
      }
      fired={fired}
      sentence={words.nothing}
      choiceLabel={words.does}
      repeats={words.repeats}
      closeLabel={texts.close}
      onClose={onClose}
    >
      {children}
    </Stage>
  );
}
