import { type CSSProperties, type ReactNode, useEffect } from "react";
import {
  DISPLAY_TESTS,
  LED_TESTS,
  restartBoardTest,
  setBoardTestDisplay,
  setBoardTestLeds,
  startBoardTest,
  stopBoardTest,
  testLed,
  useBoardTest,
} from "../boardTest";
import { Row, Section, Segmented } from "../controls";
import { PAD_COUNT } from "../customPages";
import { useDisplayFrame } from "../display";
import { Cap, Drezimo } from "../Drezimo";
import { useTexts } from "../i18n";
import { CheckIcon } from "../icons";
import type { Status } from "../status";

/** For a board just built: its keys, knob, LEDs and display, one by one */
export function BoardTestCard({ status }: { status: Status }) {
  const { test: texts, updatingBoard } = useTexts();
  const test = useBoardTest();
  const frame = useDisplayFrame();
  const updating = status.firmwareUpdate !== null && status.firmwareUpdate.state !== "failed";
  const connected = status.board.state === "connected" && !updating;
  const away = updating ? updatingBoard : texts.connectFirst;

  // Away from this page, the board plays again
  useEffect(() => stopBoardTest, []);

  if (!test) {
    return (
      <Section title={texts.title}>
        <Row label={texts.intro} text={connected ? texts.introText : away}>
          <button type="button" className="button" disabled={!connected} onClick={startBoardTest}>
            {texts.start}
          </button>
        </Row>
      </Section>
    );
  }

  const cap = (index: number) => (
    <Cap led={testLed(test, index)} pressed={test.held[index]}>
      {test.tested[index] && <CheckIcon />}
    </Cap>
  );
  const { knob } = test;
  const tested = test.tested.filter(Boolean).length;
  const done = tested === test.tested.length && knob.left && knob.right && knob.pressed;
  const started = tested > 0 || knob.left || knob.right || knob.pressed;
  const progress = { "--progress": `${(tested / test.tested.length) * 100}%` } as CSSProperties;

  return (
    <section className="card board-test">
      <header>
        <h2>{texts.title}</h2>
        <button type="button" className="button secondary" disabled={!started} onClick={restartBoardTest}>
          {texts.restart}
        </button>
        <button type="button" className="button" onClick={stopBoardTest}>
          {texts.stop}
        </button>
      </header>
      {!connected && <p className="note">{updating ? updatingBoard : texts.reconnect}</p>}

      <div className="board-test-body">
        <Drezimo
          className="board-test-drawing"
          frame={frame}
          knobHeld={knob.held}
          keys={Array.from({ length: test.tested.length - PAD_COUNT }, (_, i) => cap(PAD_COUNT + i))}
          pads={Array.from({ length: PAD_COUNT }, (_, i) => cap(i))}
        />
        <div className="board-test-checks">
          <div>
            <p className="check-title">
              {texts.keys}
              <span className="muted">{texts.keysDone(tested, test.tested.length)}</span>
            </p>
            <div className="progress" style={progress} />
            <p className="muted">{texts.keysText}</p>
          </div>
          <div>
            <p className="check-title">{texts.knob}</p>
            <ul className="checks">
              <Check done={knob.left}>{texts.left}</Check>
              <Check done={knob.right}>{texts.right}</Check>
              <Check done={knob.pressed}>{texts.press}</Check>
            </ul>
          </div>
          {done && (
            <p className="done-note">
              <CheckIcon />
              {texts.done}
            </p>
          )}
        </div>
      </div>

      <div className="rows">
        <Row label={texts.leds} text={test.leds === "keys" ? texts.ledsKeysText : texts.ledsFillText}>
          <Segmented
            label={texts.leds}
            options={LED_TESTS.map((value) => ({ value, label: texts.ledOptions[value] }))}
            value={test.leds}
            onChange={setBoardTestLeds}
          />
        </Row>
        <Row label={texts.display} text={texts.displayText[test.display]}>
          <Segmented
            label={texts.display}
            options={DISPLAY_TESTS.map((value) => ({ value, label: texts.displayOptions[value] }))}
            value={test.display}
            onChange={setBoardTestDisplay}
          />
        </Row>
      </div>
    </section>
  );
}

function Check({ done, children }: { done: boolean; children: ReactNode }) {
  return (
    <li className={done ? "done" : undefined}>
      <span className="tick">{done && <CheckIcon />}</span>
      {children}
    </li>
  );
}
