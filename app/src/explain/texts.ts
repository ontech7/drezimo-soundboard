import { TEXTS, useTexts } from "../i18n";
import type { KindMode } from "../pressAgain";
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
  borrowed: {
    doubleTap: (action: string) => string;
    longPress: (action: string) => string;
    /** Two taps on the menu: the second one picks the page it shows */
    menu: string;
    /** Two taps on a macro: the second one stops it, or starts it over */
    macro: { stop: string; restart: string };
  };
  /** The right half's captions */
  onPage: (name: string) => string;
  onRow: (row: number) => string;
  onVoice: (name: string) => string;
  menuOpen: string;
  menuPicked: string;
  allStopped: string;
  macroDone: string;
  macroStopped: string;
  volumesBack: string;
  overlayShown: string;
  overlayHidden: string;
  /** The action wheel: the slice turned to, the one done, opened at its centre */
  wheelOn: (name: string) => string;
  wheelDone: (name: string) => string;
  wheelOpen: string;
  /** Why the board does nothing with the macro or the page chosen */
  idle: { noMacro: string; noPage: string; emptyMacro: string };
  /** Drezimo's display */
  pageCounter: (page: number, pages: number) => string;
  menuTitle: string;
  volumes: { soundboardForOthers: string; soundboardForMe: string; soundboardForEveryone: string; microphone: string };
  switches: Record<"muteMic" | "muteForMe" | "voiceChanger" | "hearMyself" | "background", [on: string, off: string]>;
  resetLabels: [string, string, string];
  sample: {
    pages: string[];
    voices: string[];
    sounds: [string, string, string];
    macro: { name: string; steps: string[] };
  };
  pad: {
    captions: Record<"pad" | "press" | "again" | "againShort" | "againMore" | "hold" | "letGo", string>;
    sound: Record<
      "otherPlaying" | "playing" | "restarts" | "copies" | "stops" | "allStop" | "paused" | "resumes" | "stopsOnRelease",
      string
    >;
    sentences: Record<"restart" | "overlap" | "stop" | "pause" | "hold", string>;
    asInVoicemod: string;
    stopsAll: string;
    instead: string;
    restartBadge: string;
    pauseBadge: string;
    copyNote: string;
    otherNote: string;
    /** Which press it is */
    counts: [string, string, string];
    /** A sample sound of each type */
    sounds: Record<KindMode, string>;
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
      soundboardForEveryone:
        "Each detent turns up or down the sounds for you and for the others together: the next sound starts where you left it.",
      playingSound:
        "Each detent turns up or down the sound playing now, only that one; it keeps that volume for its next plays.",
      microphone: "Each detent turns your microphone up or down.",
      voice: "Each detent loads the next voice; turning it left goes back to the one before.",
      wheel:
        "Opens the action wheel on the screen and moves through its slices, the centre between the last and the first. Turned while held, letting go does the slice it is on; otherwise a tap does it.",
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
      overlay:
        "Hides the overlay that is always on screen, and shows it again the next time. With a duration, shows it for that long.",
      resetVolumes: "Puts the volumes changed with the knob back as they are in Voicemod.",
      wheel:
        "Opens the action wheel on the screen at its centre: turn to a slice and tap to do it. It closes by itself after 8 s.",
      nothing: "Nothing happens.",
    },
  },
  borrowed: {
    doubleTap: (action) => `The double tap has no action of its own: it is two taps, so “${action}” happens twice.`,
    longPress: (action) => `The long press has no action of its own: it counts as a tap, so it does “${action}”.`,
    menu: "The double tap has no action of its own: it is two taps. The first opens the menu, the second picks the page it shows and closes it.",
    macro: {
      stop: "The double tap has no action of its own: it is two taps. The first starts the macro, the second stops it.",
      restart:
        "The double tap has no action of its own: it is two taps. The first starts the macro, the second starts it over.",
    },
  },
  onPage: (name) => `Page ${name}`,
  onRow: (row) => `Row ${row} of 6`,
  onVoice: (name) => `Voice ${name}`,
  menuOpen: "Menu open",
  menuPicked: "Page picked, menu closed",
  allStopped: "All stopped",
  macroDone: "Macro done",
  macroStopped: "Macro stopped",
  volumesBack: "Voicemod's volumes",
  overlayShown: "Overlay shown",
  overlayHidden: "Overlay hidden",
  wheelOn: (name) => `On ${name}`,
  wheelDone: (name) => `${name}: done`,
  wheelOpen: "Wheel open",
  idle: {
    noMacro: "No macro chosen yet: until you choose one, nothing happens.",
    noPage: "No page chosen yet: until you choose one, nothing happens.",
    emptyMacro: "This macro has no steps yet: for now, nothing happens.",
  },
  pageCounter: (page, pages) => `PAGE ${page}/${pages}`,
  menuTitle: "SOUNDBOARDS",
  volumes: {
    soundboardForOthers: "Soundboard, for others",
    soundboardForMe: "Soundboard, for you",
    soundboardForEveryone: "Soundboard, for everyone",
    microphone: "Microphone",
  },
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
  pad: {
    captions: {
      pad: "Pad",
      press: "Press",
      again: "Press again while it plays",
      againShort: "Press again",
      againMore: "Press once more",
      hold: "Hold it down…",
      letGo: "…and let go",
    },
    sound: {
      otherPlaying: "Another sound is playing",
      playing: "Playing",
      restarts: "Starts over",
      copies: "Two copies together",
      stops: "Stops",
      allStop: "They all stop",
      paused: "Paused",
      resumes: "Goes on from there",
      stopsOnRelease: "Stops when you let go",
    },
    sentences: {
      restart: "Press it again while it plays: it starts over.",
      overlap: "Press it again while it plays: another copy starts, over the one already playing.",
      stop: "Press it again while it plays: it stops.",
      pause: "Press it again: it pauses, and the pad breathes. Press it once more: it goes on from where it was.",
      hold: "It plays while you hold the pad down; when you let go it stops.",
    },
    asInVoicemod: "As in Voicemod.",
    stopsAll: "Every other sound stops with it: Voicemod cannot stop just one.",
    instead: "Voicemod cannot do your choice with this kind of sound, so the pad does this.",
    restartBadge: "↺ from the start",
    pauseBadge: "❙❙ paused",
    copyNote: "copy 2",
    otherNote: "another pad",
    counts: ["1st", "2nd", "3rd"],
    sounds: { PlayRestart: "Drumroll", PlayPause: "Song", PlayStop: "Theme", PlayLoopOnPress: "Siren", PlayOverlap: "Laugh" },
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
      soundboardForEveryone:
        "Ogni scatto alza o abbassa insieme i suoni per te e per gli altri: il suono dopo parte da dove l'hai lasciato.",
      playingSound:
        "Ogni scatto alza o abbassa il volume del suono che sta suonando, solo di quello; lo tiene anche per le volte dopo.",
      microphone: "Ogni scatto alza o abbassa il volume del tuo microfono.",
      voice: "Ogni scatto carica la voce dopo; girandola a sinistra torni a quella prima.",
      wheel:
        "Apre la ruota delle azioni sullo schermo e ne scorre gli spicchi, con il centro tra l'ultimo e il primo. Girata tenendola premuta, lasciandola fa lo spicchio su cui sei; altrimenti lo fa un tocco.",
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
      overlay:
        "Nasconde l'overlay sempre sullo schermo, e la volta dopo lo rimostra. Con una durata, lo mostra per quel tempo.",
      resetVolumes: "Rimette i volumi cambiati con la manopola come sono in Voicemod.",
      wheel:
        "Apre la ruota delle azioni sullo schermo, al centro: gira fino a uno spicchio e tocca per farlo. Si chiude da sola dopo 8 s.",
      nothing: "Non succede niente.",
    },
  },
  borrowed: {
    doubleTap: (action) => `Il doppio tocco non ha un'azione sua: sono due tocchi, quindi «${action}» succede due volte.`,
    longPress: (action) => `La pressione lunga non ha un'azione sua: conta come un tocco, quindi fa «${action}».`,
    menu: "Il doppio tocco non ha un'azione sua: sono due tocchi. Il primo apre il menu, il secondo sceglie la pagina evidenziata e lo chiude.",
    macro: {
      stop: "Il doppio tocco non ha un'azione sua: sono due tocchi. Il primo avvia la macro, il secondo la ferma.",
      restart:
        "Il doppio tocco non ha un'azione sua: sono due tocchi. Il primo avvia la macro, il secondo la fa ripartire da capo.",
    },
  },
  onPage: (name) => `Pagina ${name}`,
  onRow: (row) => `Riga ${row} di 6`,
  onVoice: (name) => `Voce ${name}`,
  menuOpen: "Menu aperto",
  menuPicked: "Pagina scelta, menu chiuso",
  allStopped: "Tutti fermi",
  macroDone: "Macro finita",
  macroStopped: "Macro fermata",
  volumesBack: "Volumi di Voicemod",
  overlayShown: "Overlay visibile",
  overlayHidden: "Overlay nascosto",
  wheelOn: (name) => `Su ${name}`,
  wheelDone: (name) => `${name}: fatto`,
  wheelOpen: "Ruota aperta",
  idle: {
    noMacro: "Nessuna macro scelta: finché non ne scegli una, non succede niente.",
    noPage: "Nessuna pagina scelta: finché non ne scegli una, non succede niente.",
    emptyMacro: "Questa macro non ha ancora passi: per ora non succede niente.",
  },
  pageCounter: (page, pages) => `PAGINA ${page}/${pages}`,
  menuTitle: "SOUNDBOARD",
  volumes: {
    soundboardForOthers: "Soundboard, per gli altri",
    soundboardForMe: "Soundboard, per te",
    soundboardForEveryone: "Soundboard, per tutti",
    microphone: "Microfono",
  },
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
  pad: {
    captions: {
      pad: "Pad",
      press: "Premi",
      again: "Ripremi mentre suona",
      againShort: "Ripremi",
      againMore: "Ripremi ancora",
      hold: "Tieni premuto…",
      letGo: "…e lascia",
    },
    sound: {
      otherPlaying: "Un altro suono sta suonando",
      playing: "Suona",
      restarts: "Riparte da capo",
      copies: "Due copie insieme",
      stops: "Si ferma",
      allStop: "Si fermano tutti",
      paused: "In pausa",
      resumes: "Riprende da dove era",
      stopsOnRelease: "Si ferma quando lo lasci",
    },
    sentences: {
      restart: "Lo ripremi mentre suona: riparte da capo.",
      overlap: "Lo ripremi mentre suona: parte un'altra copia, sopra quella che suona già.",
      stop: "Lo ripremi mentre suona: si ferma.",
      pause: "Lo ripremi: va in pausa, e il pad respira. Lo ripremi ancora: riprende da dove era.",
      hold: "Suona finché tieni premuto il pad; quando lo lasci si ferma.",
    },
    asInVoicemod: "Come in Voicemod.",
    stopsAll: "Con lui si fermano anche tutti gli altri suoni: Voicemod non sa fermarne uno solo.",
    instead: "Voicemod non sa fare la tua scelta con questo tipo di suono, quindi il pad fa così.",
    restartBadge: "↺ da capo",
    pauseBadge: "❙❙ pausa",
    copyNote: "copia 2",
    otherNote: "un altro pad",
    counts: ["1°", "2°", "3°"],
    sounds: {
      PlayRestart: "Rullo di tamburi",
      PlayPause: "Canzone",
      PlayStop: "Sigla",
      PlayLoopOnPress: "Sirena",
      PlayOverlap: "Risata",
    },
  },
};

/** The explanations' words in the window's language */
export function useExplainTexts(): ExplainTexts {
  return useTexts() === TEXTS.italian ? ITALIAN : ENGLISH;
}
