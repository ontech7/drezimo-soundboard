import { Row, Section } from "./controls";
import { pageTargets, useCustomPages, useLibrary } from "./customPages";
import { useTexts } from "./i18n";
import { useMacros } from "./macros";
import { SearchSelect } from "./SearchSelect";
import { type KnobSettings, type PressAction, type SettingsView, saveSettings, type TurnAction } from "./settings";
import type { Status } from "./status";

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
/** What goes through Voicemod's bridge (src-tauri/src/voicemod_bridge.rs) */
const BRIDGE_ACTIONS = new Set(["soundboardForOthers", "soundboardForMe", "playingSound", "microphone", "resetVolumes"]);

type TurnGesture = "turn" | "holdTurn";
type PressGesture = "tap" | "doubleTap" | "longPress";

/** What each of the knob's gestures does, on the Drezimo page */
export function KnobTab({ settings, status }: { settings: SettingsView; status: Status }) {
  const texts = useTexts();
  const knobTexts = texts.knobTab;
  const knob = settings.knob;
  const bridge = status.singleSoundStop;
  const library = useLibrary();
  const [pages] = useCustomPages();
  const [macros] = useMacros();
  const targets = pageTargets(pages ?? [], library, texts);

  const save = <G extends keyof KnobSettings>(gesture: G, choice: KnobSettings[G]) =>
    saveSettings({ ...settings, knob: { ...knob, [gesture]: choice } });
  const label = (action: string, name: string) =>
    !bridge && BRIDGE_ACTIONS.has(action) ? `${name} · ${knobTexts.notNow}` : name;
  const chosen = [knob.turn, knob.holdTurn, knob.tap, knob.doubleTap, knob.longPress].map((choice) => choice.action);
  const usesVolumes = chosen.some((action) => BRIDGE_ACTIONS.has(action));

  const turnRow = (gesture: TurnGesture) => (
    <Row key={gesture} label={knobTexts.gestures[gesture]}>
      <div className="knob-select">
        <SearchSelect
          label={knobTexts.gestures[gesture]}
          options={TURN_ACTIONS.map((action) => ({ value: action, label: label(action, knobTexts.turnActions[action]) }))}
          value={knob[gesture].action}
          onChange={(action) => save(gesture, { action })}
          align="right"
        />
      </div>
    </Row>
  );

  const pressRow = (gesture: PressGesture, hint?: string) => {
    const choice = knob[gesture];
    const pick = (action: PressAction["action"]) => {
      if (action === "macro") save(gesture, { action, macroId: macros?.[0]?.id ?? "" });
      else if (action === "page") save(gesture, { action, page: targets[0]?.id ?? "" });
      else save(gesture, { action });
    };
    // A target deleted since: the select shows none, and the row says why
    let gone: string | undefined;
    let target = null;
    if (choice.action === "macro") {
      const missing = macros !== null && choice.macroId !== "" && !macros.some((item) => item.id === choice.macroId);
      if (missing) gone = knobTexts.macroGone;
      target = (
        <SearchSelect
          label={knobTexts.whichMacro}
          options={(macros ?? []).map((item) => ({ value: item.id, label: item.name }))}
          value={choice.macroId}
          placeholder={knobTexts.chooseMacro}
          onChange={(macroId) => save(gesture, { action: "macro", macroId })}
          searchLabel={knobTexts.searchMacro}
          empty={knobTexts.noMacro}
          align="right"
        />
      );
    } else if (choice.action === "page") {
      const missing = library !== null && choice.page !== "" && !targets.some((page) => page.id === choice.page);
      if (missing) gone = knobTexts.pageGone;
      target = (
        <SearchSelect
          label={texts.pageToOpen}
          options={targets.map((page) => ({ value: page.id, label: page.name }))}
          value={choice.page}
          placeholder={texts.choosePage}
          onChange={(page) => save(gesture, { action: "page", page })}
          searchLabel={texts.searchPage}
          empty={texts.noPage}
          align="right"
        />
      );
    }
    return (
      <Row key={gesture} label={knobTexts.gestures[gesture]} text={gone ?? hint}>
        <div className="knob-select">
          <SearchSelect
            label={knobTexts.gestures[gesture]}
            options={PRESS_ACTIONS.map((action) => ({ value: action, label: label(action, knobTexts.pressActions[action]) }))}
            value={choice.action}
            onChange={pick}
            align="right"
          />
          {target}
        </div>
      </Row>
    );
  };

  return (
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
  );
}
