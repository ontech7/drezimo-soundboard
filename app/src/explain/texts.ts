/** The explanations' words: here rather than in i18n.ts, so that they load with them */
export interface ExplainTexts {
  repeats: string;
  /** Before the knob's select in the dialog */
  does: string;
  nothing: string;
  gestures: {
    still: string;
    turnRight: string;
    turnLeft: string;
    holdIt: string;
    turnBack: string;
    released: string;
    tap: string;
    waits: string;
    tapped: string;
    tapFirst: string;
    tapAgain: string;
    tapAgainSoon: string;
    holdLong: string;
    letGo: string;
  };
}

export type GestureCaption = keyof ExplainTexts["gestures"];

const ENGLISH: ExplainTexts = {
  repeats: "It repeats while this window is open.",
  does: "Does",
  nothing: "Nothing happens",
  gestures: {
    still: "Still",
    turnRight: "Turn it right",
    turnLeft: "Turn it left",
    holdIt: "Hold it down…",
    turnBack: "…and turn it back too",
    released: "Let go",
    tap: "Tap",
    waits: "It waits 0.3 s for a second tap…",
    tapped: "Tapped",
    tapFirst: "Tap…",
    tapAgain: "…and tap again",
    tapAgainSoon: "…and tap again within 0.3 s",
    holdLong: "Hold it down 0.6 s…",
    letGo: "…and let go",
  },
};

const ITALIAN: ExplainTexts = {
  repeats: "Si ripete finché la finestra è aperta.",
  does: "Fa",
  nothing: "Non succede niente",
  gestures: {
    still: "Ferma",
    turnRight: "Gira a destra",
    turnLeft: "Gira a sinistra",
    holdIt: "Tienila premuta…",
    turnBack: "…e girala anche indietro",
    released: "Lasciata",
    tap: "Tocca",
    waits: "Aspetta 0,3 s un secondo tocco…",
    tapped: "Tocco",
    tapFirst: "Tocca…",
    tapAgain: "…e tocca ancora",
    tapAgainSoon: "…e tocca ancora, entro 0,3 s",
    holdLong: "Tienila premuta 0,6 s…",
    letGo: "…e lasciala",
  },
};

export function explainTexts(language: "english" | "italian"): ExplainTexts {
  return language === "italian" ? ITALIAN : ENGLISH;
}
