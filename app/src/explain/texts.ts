import type { PressAction, TurnAction } from "../settings";

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
  /** What each action does, under the stage */
  sentences: { turn: Record<TurnAction["action"], string>; press: Record<PressAction["action"], string> };
  /** A double tap or a long press with no action of its own, and the tap's action they come to */
  borrowed: { doubleTap: (action: string) => string; longPress: (action: string) => string };
  /** The right half's captions */
  onPage: (name: string) => string;
  onRow: (row: number) => string;
  onVoice: (name: string) => string;
  menuOpen: string;
  allStopped: string;
  macroDone: string;
  volumesBack: string;
  /** Drezimo's display */
  pageCounter: (page: number, pages: number) => string;
  menuTitle: string;
  volumes: { soundboardForOthers: string; soundboardForMe: string; microphone: string };
  switches: Record<"muteMic" | "muteForMe" | "voiceChanger" | "hearMyself" | "background", [on: string, off: string]>;
  resetLabels: [string, string, string];
  sample: {
    pages: string[];
    voices: string[];
    sounds: [string, string, string];
    macro: { name: string; steps: string[] };
  };
}

export type GestureCaption = keyof ExplainTexts["gestures"];
export type SwitchAction = keyof ExplainTexts["switches"];

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
  sentences: {
    turn: {
      page: "Each detent moves to the next page; turning it left goes back to the one before.",
      rows: "Each detent lights the next row of pads white, or the one before: you read row by row which sounds are there.",
      soundboardForOthers: "Each detent turns up or down the sounds the others hear.",
      soundboardForMe: "Each detent turns up or down the sounds you hear, in your headphones.",
      playingSound: "Each detent turns up or down the sound playing now, only that one.",
      microphone: "Each detent turns your microphone up or down.",
      voice: "Each detent loads the next voice; turning it left goes back to the one before.",
      nothing: "Turning it does nothing.",
    },
    press: {
      menu: "Opens the soundboard menu on the display, to pick one.",
      stopSounds: "Stops every sound playing, at once.",
      muteMic: "Mutes the microphone; the next time it turns it back on.",
      muteForMe: "You stop hearing the sounds, the others still hear them. The next time you hear them again.",
      voiceChanger: "Turns the voice changer off, so you speak with your own voice; the next time it turns it back on.",
      hearMyself: "You stop hearing your changed voice in your headphones; the next time you hear it again.",
      background: "Turns the voice's background effects off; the next time it turns them back on.",
      randomVoice: "Loads a random voice among yours.",
      macro: "Runs the chosen macro, one step after the other.",
      page: "Goes straight to the chosen page, from whichever page you are on.",
      resetVolumes: "Puts the volumes changed with the knob back as they are in Voicemod.",
      nothing: "Nothing happens.",
    },
  },
  borrowed: {
    doubleTap: (action) => `The double tap has no action of its own: it is two taps, so “${action}” happens twice.`,
    longPress: (action) => `The long press has no action of its own: it counts as a tap, so it does “${action}”.`,
  },
  onPage: (name) => `Page ${name}`,
  onRow: (row) => `Row ${row} of 6`,
  onVoice: (name) => `Voice ${name}`,
  menuOpen: "Menu open",
  allStopped: "All stopped",
  macroDone: "Macro done",
  volumesBack: "Voicemod's volumes",
  pageCounter: (page, pages) => `PAGE ${page}/${pages}`,
  menuTitle: "SOUNDBOARDS",
  volumes: { soundboardForOthers: "Soundboard, for others", soundboardForMe: "Soundboard, for you", microphone: "Microphone" },
  switches: {
    muteMic: ["Microphone on", "Microphone muted"],
    muteForMe: ["You hear the sounds", "You don't hear the sounds"],
    voiceChanger: ["Voice changer on", "Voice changer off"],
    hearMyself: ["You hear yourself", "You don't hear yourself"],
    background: ["Background on", "Background off"],
  },
  resetLabels: ["For others", "For you", "Microphone"],
  sample: {
    pages: ["Memes", "Laughs", "Music", "Effects"],
    voices: ["Clean", "Robot", "Echo", "Baby", "Demon", "Radio", "Giant"],
    sounds: ["Drumroll", "Applause", "Song"],
    macro: { name: "Grand entrance", steps: ["Voice Robot", "Sound Drumroll", "Wait 1 s", "Sound Applause"] },
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
  sentences: {
    turn: {
      page: "Ogni scatto passa alla pagina dopo; girandola a sinistra torni a quella prima.",
      rows: "Ogni scatto accende di bianco la riga di pad dopo, o quella prima: così leggi riga per riga quali suoni ci sono.",
      soundboardForOthers: "Ogni scatto alza o abbassa il volume dei suoni che sentono gli altri.",
      soundboardForMe: "Ogni scatto alza o abbassa il volume dei suoni che senti tu, in cuffia.",
      playingSound: "Ogni scatto alza o abbassa il volume del suono che sta suonando, solo di quello.",
      microphone: "Ogni scatto alza o abbassa il volume del tuo microfono.",
      voice: "Ogni scatto carica la voce dopo; girandola a sinistra torni a quella prima.",
      nothing: "Girandola non succede niente.",
    },
    press: {
      menu: "Apre sul display il menu delle soundboard, per sceglierne una.",
      stopSounds: "Ferma tutti i suoni che stanno suonando, in un colpo.",
      muteMic: "Spegne il microfono; la volta dopo lo riaccende.",
      muteForMe: "Smetti di sentire i suoni tu; gli altri continuano a sentirli. La volta dopo torni a sentirli.",
      voiceChanger: "Spegne il cambio voce, così parli con la tua voce; la volta dopo lo riaccende.",
      hearMyself: "Smetti di sentire in cuffia la tua voce cambiata; la volta dopo torni a sentirla.",
      background: "Spegne gli effetti di sottofondo della voce; la volta dopo li riaccende.",
      randomVoice: "Carica una voce a caso tra le tue.",
      macro: "Esegue la macro scelta, un passo dopo l'altro.",
      page: "Va dritta alla pagina scelta, da qualunque pagina tu sia.",
      resetVolumes: "Rimette i volumi cambiati con la manopola come sono in Voicemod.",
      nothing: "Non succede niente.",
    },
  },
  borrowed: {
    doubleTap: (action) => `Il doppio tocco non ha un'azione sua: sono due tocchi, quindi «${action}» succede due volte.`,
    longPress: (action) => `La pressione lunga non ha un'azione sua: conta come un tocco, quindi fa «${action}».`,
  },
  onPage: (name) => `Pagina ${name}`,
  onRow: (row) => `Riga ${row} di 6`,
  onVoice: (name) => `Voce ${name}`,
  menuOpen: "Menu aperto",
  allStopped: "Tutti fermi",
  macroDone: "Macro finita",
  volumesBack: "Volumi di Voicemod",
  pageCounter: (page, pages) => `PAGINA ${page}/${pages}`,
  menuTitle: "SOUNDBOARD",
  volumes: { soundboardForOthers: "Soundboard, per gli altri", soundboardForMe: "Soundboard, per te", microphone: "Microfono" },
  switches: {
    muteMic: ["Microfono attivo", "Microfono muto"],
    muteForMe: ["Senti i suoni", "Non senti i suoni"],
    voiceChanger: ["Cambio voce acceso", "Cambio voce spento"],
    hearMyself: ["Ti senti", "Non ti senti"],
    background: ["Sottofondo acceso", "Sottofondo spento"],
  },
  resetLabels: ["Per gli altri", "Per te", "Microfono"],
  sample: {
    pages: ["Meme", "Risate", "Musica", "Effetti"],
    voices: ["Normale", "Robot", "Eco", "Bambino", "Demone", "Radio", "Gigante"],
    sounds: ["Rullo di tamburi", "Applausi", "Canzone"],
    macro: { name: "Entrata in scena", steps: ["Voce Robot", "Suono Rullo di tamburi", "Attendi 1 s", "Suono Applausi"] },
  },
};

export function explainTexts(language: "english" | "italian"): ExplainTexts {
  return language === "italian" ? ITALIAN : ENGLISH;
}
