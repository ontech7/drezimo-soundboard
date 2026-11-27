import type { ReactNode } from "react";
import type { Language } from "../settings";

/** The scenes a version's news can show (`WhatsNew.tsx`) */
export type Scene = "macroTogether" | "ledsAtRest" | "displayOnScreen" | "knobTimes" | "pagesNoGaps";

export interface Highlight {
  scene: Scene;
  title: string;
  text: ReactNode;
}

/** What is new in a version, as the window tells it once the app updated itself */
export interface Changelog {
  highlights: Highlight[];
  fixesTitle: string;
  fixes: ReactNode[];
  /** The words drawn in the scenes */
  words: SceneWords;
}

export interface SceneWords {
  track: string;
  loop: string;
  stopsOthers: string;
  mutesOthers: string;
  overEachOther: string;
  putBack: string;
  solid: string;
  breathe: string;
  blink: string;
  never: string;
  always: string;
  withActions: string;
  seconds: (seconds: number) => string;
  untilNow: string;
  asYouSet: string;
  startFrom: string;
}

type Shown = Exclude<Language, "system">;

const V1_2_0: Record<Shown, Changelog> = {
  english: {
    highlights: [
      {
        scene: "macroTogether",
        title: "A macro's sounds play together",
        text: (
          <>
            Sounds on the time line now <strong>play over each other, once each</strong>, whatever you set for them in Voicemod:{" "}
            <em>loop</em>, <em>stop the other sounds</em> and <em>mute the other sounds</em> are set aside for the macro and{" "}
            <strong>put back</strong> right after. <strong>Stopping a macro stops its sounds</strong> too.
          </>
        ),
      },
      {
        scene: "ledsAtRest",
        title: "Pads that breathe or blink",
        text: (
          <>
            A pad of your pages and a macro can now <strong>breathe</strong> or <strong>blink</strong> at rest, in one of{" "}
            <strong>14 colours</strong>. A running macro breathes, or blinks if it already breathes. Pick it under{" "}
            <em>Movement</em>, from the <em>LED colour</em> button in the Pages editor or on the macro.
          </>
        ),
      },
      {
        scene: "displayOnScreen",
        title: "Drezimo's display on your screen",
        text: (
          <>
            From the <em>Overlay</em> page, Drezimo's display shows on the PC screen <strong>never</strong>,{" "}
            <strong>always</strong>, or <strong>with some actions</strong>: the soundboard menu, a volume, the knob held, a page
            change. It sits <strong>beside the overlay</strong>, or where you put it. <em>Show it now</em> shows it for a
            moment.
          </>
        ),
      },
      {
        scene: "knobTimes",
        title: "More time for the knob",
        text: (
          <>
            On the <em>Knob</em> tab you set how long a <strong>double tap</strong> waits for the second tap (<kbd>0.2</kbd> to{" "}
            <kbd>1 s</kbd>) and how long a <strong>long press</strong> lasts (<kbd>0.4</kbd> to <kbd>2 s</kbd>). The knob's
            explanations follow your times.
          </>
        ),
      },
      {
        scene: "pagesNoGaps",
        title: "Pages without gaps",
        text: (
          <>
            Turn on <em>No empty pads</em> in a page's <kbd>⋯</kbd> menu and the pads <strong>close up</strong> when you clear
            one. A page started from a long soundboard takes <strong>the part you pick</strong>, and the soundboard menu on the
            knob <strong>goes round</strong> past its ends.
          </>
        ),
      },
    ],
    fixesTitle: "And these fixes",
    fixes: [
      <>
        A volume point at <strong>0 %</strong> on a clip can be grabbed again.
      </>,
      <>
        Options a macro changed are <strong>put back reliably</strong>, even if the app closes halfway, and given up for a sound
        you deleted in Voicemod.
      </>,
      <>
        A sound that would <strong>mute the others</strong> no longer silences the macro's next sound for a moment.
      </>,
      <>
        The display's <strong>size and place</strong> on the Overlay page match what is drawn.
      </>,
    ],
    words: {
      track: "Track",
      loop: "loop",
      stopsOthers: "stops the others",
      mutesOthers: "mutes the others",
      overEachOther: "over each other, once each",
      putBack: "Voicemod's options back after",
      solid: "Steady",
      breathe: "Breathing",
      blink: "Blinking",
      never: "Never",
      always: "Always",
      withActions: "With some actions",
      seconds: (seconds) => `${seconds} s`,
      untilNow: "long press, until now",
      asYouSet: "long press, as you set it",
      startFrom: "Start from a soundboard:",
    },
  },
  italian: {
    highlights: [
      {
        scene: "macroTogether",
        title: "I suoni di una macro suonano insieme",
        text: (
          <>
            I suoni sulla linea del tempo ora <strong>suonano uno sopra l'altro, una volta ciascuno</strong>, qualunque cosa tu
            abbia impostato in Voicemod: <em>ripeti</em>, <em>ferma gli altri suoni</em> e <em>silenzia gli altri suoni</em>{" "}
            vengono messi da parte per la macro e <strong>rimessi</strong> subito dopo.{" "}
            <strong>Fermare una macro ferma anche i suoi suoni</strong>.
          </>
        ),
      },
      {
        scene: "ledsAtRest",
        title: "Pad che respirano o lampeggiano",
        text: (
          <>
            Un pad delle tue pagine e una macro ora possono <strong>respirare</strong> o <strong>lampeggiare</strong> a riposo,
            in uno di <strong>14 colori</strong>. Una macro in corso respira, o lampeggia se respira già. Lo scegli in{" "}
            <em>Animazione</em>, dal pulsante <em>Colore del LED</em> nell'editor delle pagine o sulla macro.
          </>
        ),
      },
      {
        scene: "displayOnScreen",
        title: "Il display di Drezimo sul tuo schermo",
        text: (
          <>
            Dalla pagina <em>Overlay</em>, il display di Drezimo compare sullo schermo del PC <strong>mai</strong>,{" "}
            <strong>sempre</strong>, oppure <strong>con alcune azioni</strong>: il menu delle soundboard, un volume, la manopola
            tenuta premuta, un cambio pagina. Sta <strong>accanto all'overlay</strong>, o dove lo metti tu.{" "}
            <em>Mostralo ora</em> lo mostra per un momento.
          </>
        ),
      },
      {
        scene: "knobTimes",
        title: "Più tempo per la manopola",
        text: (
          <>
            Nella scheda <em>Manopola</em> imposti quanto un <strong>doppio tocco</strong> aspetta il secondo tocco (da{" "}
            <kbd>0,2</kbd> a <kbd>1 s</kbd>) e quanto dura una <strong>pressione lunga</strong> (da <kbd>0,4</kbd> a{" "}
            <kbd>2 s</kbd>). Le spiegazioni della manopola seguono i tuoi tempi.
          </>
        ),
      },
      {
        scene: "pagesNoGaps",
        title: "Pagine senza buchi",
        text: (
          <>
            Attiva <em>Niente pad vuoti</em> nel menu <kbd>⋯</kbd> di una pagina e i pad <strong>si ricompattano</strong> quando
            ne svuoti uno. Una pagina creata da una soundboard lunga prende <strong>la parte che scegli</strong>, e il menu
            delle soundboard sulla manopola <strong>ricomincia dall'inizio</strong> dopo l'ultima.
          </>
        ),
      },
    ],
    fixesTitle: "E queste correzioni",
    fixes: [
      <>
        Un punto del volume a <strong>0 %</strong> su una clip si può di nuovo afferrare.
      </>,
      <>
        Le opzioni cambiate da una macro vengono <strong>rimesse in modo affidabile</strong>, anche se l'app si chiude a metà, e
        lasciate perdere per un suono che hai eliminato in Voicemod.
      </>,
      <>
        Un suono che <strong>silenzia gli altri</strong> non zittisce più per un momento il suono successivo della macro.
      </>,
      <>
        La <strong>dimensione e la posizione</strong> del display nella pagina Overlay corrispondono a quello che si vede.
      </>,
    ],
    words: {
      track: "Traccia",
      loop: "ripeti",
      stopsOthers: "ferma gli altri",
      mutesOthers: "silenzia gli altri",
      overEachOther: "uno sopra l'altro, una volta ciascuno",
      putBack: "opzioni di Voicemod rimesse dopo",
      solid: "Fissa",
      breathe: "Respiro",
      blink: "Lampeggio",
      never: "Mai",
      always: "Sempre",
      withActions: "Con alcune azioni",
      seconds: (seconds) => `${String(seconds).replace(".", ",")} s`,
      untilNow: "pressione lunga, finora",
      asYouSet: "pressione lunga, come la imposti",
      startFrom: "Parti da una soundboard:",
    },
  },
};

/** By version, in each language the window speaks: a version without news shows only the line of its release */
const CHANGELOGS: Record<string, Record<Shown, Changelog>> = { "1.2.0": V1_2_0 };

export function hasChangelog(version: string): boolean {
  return version in CHANGELOGS;
}

export function changelog(version: string, language: Shown): Changelog | null {
  return CHANGELOGS[version]?.[language] ?? null;
}
