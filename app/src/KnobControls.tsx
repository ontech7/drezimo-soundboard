import { lazy, type ReactNode, Suspense, useState } from "react";
import { Row, Section } from "./controls";
import { pageTargets, useCustomPages, useLibrary } from "./customPages";
import { useTexts } from "./i18n";
import { BRIDGE_ACTIONS, goneTarget } from "./knob";
import { useMacros } from "./macros";
import { SearchSelect } from "./SearchSelect";
import { type KnobSettings, type PressAction, type SettingsView, saveSettings, type TurnAction } from "./settings";
import type { Status } from "./status";
import type { KnobGesture } from "./explain/story";

const TURN_ACTIONS: TurnAction["action"][] = [
  "page",
  "rows",
  "soundboardForOthers",
  "soundboardForMe",
  "playingSound",
  "microphone",
  "voice",
  "nothing",
];
const PRESS_ACTIONS: PressAction["action"][] = [
  "menu",
  "stopSounds",
  "muteMic",
  "muteForMe",
  "voiceChanger",
  "hearMyself",
  "background",
  "randomVoice",
  "macro",
  "page",
  "resetVolumes",
  "nothing",
];

type TurnGesture = "turn" | "holdTurn";
type PressGesture = "tap" | "doubleTap" | "longPress";

const KnobExplainer = lazy(() => import("./explain/Explainer").then((module) => ({ default: module.KnobExplainer })));

/** What each of the knob's gestures does, on the Drezimo page */
export function KnobTab({ settings, status }: { settings: SettingsView; status: Status }) {
  const texts = useTexts();
  const knobTexts = texts.knobTab;
  const knob = settings.knob;
  const bridge = status.singleSoundStop;
  const library = useLibrary();
  const [pages] = useCustomPages();
  const [macros] = useMacros();
  const [explained, setExplained] = useState<KnobGesture | null>(null);
  const targets = pageTargets(pages ?? [], library, texts);

  const save = <G extends keyof KnobSettings>(gesture: G, choice: KnobSettings[G]) =>
    saveSettings({ ...settings, knob: { ...knob, [gesture]: choice } });
  const unavailable = (action: string) => !bridge && BRIDGE_ACTIONS.has(action);
  // The chosen one is marked by its border, so that its name fits on the button
  const label = (action: string, name: string, chosen: string) =>
    unavailable(action) && action !== chosen ? `${name} · ${knobTexts.notNow}` : name;
  const selectClass = (action: string) => (unavailable(action) ? "knob-select unavailable" : "knob-select");
  const chosen = [knob.turn, knob.holdTurn, knob.tap, knob.doubleTap, knob.longPress].map((choice) => choice.action);
  const usesVolumes = chosen.some((action) => BRIDGE_ACTIONS.has(action));
  // The same selects in a row and in its explanation, whose foot leaves room for their menus only above
  const placed = (inDialog: boolean) => ({ align: inDialog ? ("left" as const) : ("right" as const), above: inDialog });

  const turnControl = (gesture: TurnGesture, inDialog: boolean) => (
    <div className={selectClass(knob[gesture].action)}>
      <SearchSelect
        label={knobTexts.gestures[gesture]}
        options={TURN_ACTIONS.map((action) => ({
          value: action,
          label: label(action, knobTexts.turnActions[action], knob[gesture].action),
        }))}
        value={knob[gesture].action}
        onChange={(action) => save(gesture, { action })}
        {...placed(inDialog)}
      />
    </div>
  );

  const pressControl = (gesture: PressGesture, inDialog: boolean): { control: ReactNode; gone?: string } => {
    const choice = knob[gesture];
    const pick = (action: PressAction["action"]) => {
      if (action === "macro") save(gesture, { action, macroId: macros?.[0]?.id ?? "" });
      else if (action === "page") save(gesture, { action, page: targets[0]?.id ?? "" });
      else save(gesture, { action });
    };
    // A target deleted since: the select shows none, and the row says why
    const gone = goneTarget(choice, macros, targets, library, texts);
    let target = null;
    if (choice.action === "macro") {
      target = (
        <SearchSelect
          label={knobTexts.whichMacro}
          options={(macros ?? []).map((item) => ({ value: item.id, label: item.name }))}
          value={choice.macroId}
          placeholder={knobTexts.chooseMacro}
          onChange={(macroId) => save(gesture, { action: "macro", macroId })}
          searchLabel={knobTexts.searchMacro}
          empty={knobTexts.noMacro}
          {...placed(inDialog)}
        />
      );
    } else if (choice.action === "page") {
      target = (
        <SearchSelect
          label={texts.pageToOpen}
          options={targets.map((page) => ({ value: page.id, label: page.name }))}
          value={choice.page}
          placeholder={texts.choosePage}
          onChange={(page) => save(gesture, { action: "page", page })}
          searchLabel={texts.searchPage}
          empty={texts.noPage}
          {...placed(inDialog)}
        />
      );
    }
    const control = (
      <div className={selectClass(choice.action)}>
        <SearchSelect
          label={knobTexts.gestures[gesture]}
          options={PRESS_ACTIONS.map((action) => ({
            value: action,
            label: label(action, knobTexts.pressActions[action], choice.action),
          }))}
          value={choice.action}
          onChange={pick}
          {...placed(inDialog)}
        />
        {target}
      </div>
    );
    return { control, gone };
  };

  const turnRow = (gesture: TurnGesture) => (
    <Row key={gesture} label={knobTexts.gestures[gesture]} onExplain={() => setExplained(gesture)}>
      {turnControl(gesture, false)}
    </Row>
  );

  const pressRow = (gesture: PressGesture, hint?: string) => {
    const { control, gone } = pressControl(gesture, false);
    return (
      <Row key={gesture} label={knobTexts.gestures[gesture]} text={gone ?? hint} onExplain={() => setExplained(gesture)}>
        {control}
      </Row>
    );
  };

  return (
    <>
      <Section title={knobTexts.title} text={knobTexts.text}>
        {turnRow("turn")}
        {turnRow("holdTurn")}
        {pressRow("tap")}
        {pressRow("doubleTap", knob.doubleTap.action === "nothing" ? undefined : knobTexts.tapWaits)}
        {pressRow("longPress", knobTexts.longPressHint)}
        <div className="knob-notes muted">
          {usesVolumes && !bridge && <p className="warning">{knobTexts.bridgeMissing}</p>}
          {usesVolumes && <p>{knobTexts.volumesNote}</p>}
          <p>{knobTexts.peekHint}</p>
        </div>
      </Section>
      {explained && (
        <Suspense fallback={null}>
          <KnobExplainer
            gesture={explained}
            settings={settings}
            bridge={bridge}
            macros={macros}
            targets={targets}
            library={library}
            onClose={() => setExplained(null)}
          >
            {explained === "turn" || explained === "holdTurn"
              ? turnControl(explained, true)
              : pressControl(explained, true).control}
          </KnobExplainer>
        </Suspense>
      )}
    </>
  );
}
