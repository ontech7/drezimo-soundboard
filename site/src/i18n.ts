export type Language = "en" | "it";

const en = {
  metaDescription: "Drezimo Soundboard: 36 pads, 6 function keys, a knob and a display for Voicemod.",
  sceneLabel: "The Drezimo in 3D",
  caseAlt: "The Drezimo's enclosure",
  languageLabel: "Language",
  released: "Version {version} is out",
  tagline: "42 keys that light up, a knob and a display. Voicemod, finally at your fingertips.",
  download: "Download for Windows",
  needs: "Free · needs Voicemod, free or PRO",
  mac: "Also for macOS (Apple Silicon)",
  hintPointer: "Click the keys, or play them from your keyboard",
  hintTouch: "Tap the keys",
  hintScroll: "Scroll to open it",
  caption1Title: "42 keys, 42 RGB LEDs",
  caption1Text: "Every key has its own LED, in the colour of its sound, lit while it plays.",
  caption2Title: "Printed in two parts",
  caption2Text: "A matte black shell with the logo printed flush in white, a ribbed bottom, six M3 screws.",
  caption3Title: "A display and a knob",
  caption3Text: "The page and the sound playing at a glance. Turn the knob to change page, hold it to see every name.",
  sounds:
    "Airhorn|Applause|Drum roll|Rimshot|Sad trombone|Bruh|Boing|Laser|Ding|Wow|Honk|Crickets|Victory|Fail|Bleep|Record scratch|Evil laugh|Ka-ching",
  stat1: "pads per page",
  stat2: "function keys",
  stat3: "RGB LEDs",
  stat4: "USB cable",
  featuresEyebrow: "01 · What it does",
  featuresTitle: "Every sound, one key away",
  feature1Title: "Voicemod's soundboards",
  feature1Text: "Your soundboards and voices arrive on the pads as pages of 36, with their pictures and colours.",
  feature2Title: "Your own pages",
  feature2Text: "Sounds, voices and actions in the order you want, with your own names and LED colours.",
  feature3Title: "Macros",
  feature3Text: "One pad, a whole time line: sounds over each other, voices and switches at their moment.",
  macroSteps: "Airhorn|wait 0.5 s|Robot voice",
  feature4Title: "Always up to date",
  feature4Text: "The app updates itself and the Drezimo's firmware, then shows what is new, in your language.",
  updateApp: "App",
  updateFirmware: "Firmware",
  updateDone: "Up to date",
  macroEyebrow: "02 · Macros",
  macroTitle: "A whole bit, on one key",
  macroLead:
    "Lay sounds on tracks along a time line, cut them, fade them, and drop voices and switches at their moment. One press on the Drezimo plays it all, sounds over each other. Try this one.",
  macroName: "Big entrance",
  macroPadLabel: "Pad 1",
  macroPlay: "Play",
  macroStop: "Stop",
  macroLength: "Lasts {value}",
  macroSeconds: "{value} s",
  macroTracks: "Tracks",
  macroTrack: "Track {n}",
  macroActions: "Actions",
  macroFade: "In and out",
  macroOverItself: "Again, over itself",
  macroVoiceOn: "Robot voice on",
  actionVoice: "Voice: Robot",
  actionBackground: "Background effects: on",
  actionBack: "Back to the voice",
  macroPadText: "Its pad breathes while it plays: press it again to stop.",
  macroPoint1Title: "Sounds over each other",
  macroPoint1Text:
    "Tracks that play together, even the same sound twice. Voicemod's own stop, mute and loop are set aside for the macro and put back after.",
  macroPoint2Title: "Cuts and fades",
  macroPoint2Text: "Cut a clip where you want and shape its volume: fade in, fade out, or points of your own.",
  macroPoint3Title: "Actions on time",
  macroPoint3Text:
    "Voices, the mic, the voice changer and background effects, each at its moment. The voice and the switches come back at the end, if you want.",
  macroPoint4Title: "Edited like a track",
  macroPoint4Text: "Undo and redo, snapping, several clips at once, nudged with the arrows.",
  macroKeys: "Ctrl+Z|Ctrl+Y|Shift+click|← →|Del",
  feature5Title: "The action wheel",
  feature5Text:
    "A knob gesture opens a wheel of up to eight shortcuts on screen: pages, actions, macros. Turn to pick. Double tap and long press take the time you set.",
  feature6Title: "Pads that breathe",
  feature6Text: "Each pad of your pages, and each macro, steady, breathing or blinking at rest, in 14 colours.",
  feature7Title: "The display, on your screen",
  feature7Text: "The Drezimo's display beside the overlay: always, or with the actions you pick.",
  app4: "The Macro editor: tracks, cuts, fades and actions",
  app5: "The action wheel, around the Drezimo's logo",
  appEyebrow: "03 · The app",
  appTitle: "Everything, live, on your screen",
  app1: "The Drezimo's page, live, with Voicemod's pictures",
  app2: "Your own pages, in the order you want",
  app3: "The overlay, over every window",
  installEyebrow: "04 · Setup",
  installTitle: "Ready in a minute",
  step1Title: "Download the installer",
  step1Text: "From the button at the top. Voicemod, free or PRO, must be on the same PC.",
  step2Title: "Confirm it once",
  step2Text: "The installer is not signed, so Windows asks: More info, then Run anyway.",
  step3Title: "Plug in the Drezimo",
  step3Text: "One USB cable. The app starts with Windows and finds it on its own.",
  encoreTitle: "Your turn.",
  encoreText: "Six pads to warm up. The real Drezimo has 36.",
  encorePad: "Play {name}",
  soundsCredit: "Sounds: Freesound, CC0",
  footer: "Made by hand: electronics, firmware, case and app.",
  source: "Source on GitHub",
  oledPage: "PAGE",
  oledReady: "READY",
  oledStop: "STOPPED",
  oledMute: "MIC MUTED",
  oledVoice: "VOICE ON",
  oledHear: "HEAR ME",
};

export type Key = keyof typeof en;

const it: Record<Key, string> = {
  metaDescription: "Drezimo Soundboard: 36 pad, 6 tasti funzione, una manopola e un display per Voicemod.",
  sceneLabel: "Il Drezimo in 3D",
  caseAlt: "Il case del Drezimo",
  languageLabel: "Lingua",
  released: "È uscita la versione {version}",
  tagline: "42 tasti che si accendono, una manopola e un display. Voicemod, finalmente sotto le dita.",
  download: "Scarica per Windows",
  needs: "Gratis · serve Voicemod, gratuito o PRO",
  mac: "Anche per macOS (Apple Silicon)",
  hintPointer: "Clicca i tasti, o suonali dalla tastiera",
  hintTouch: "Tocca i tasti",
  hintScroll: "Scorri per aprirlo",
  caption1Title: "42 tasti, 42 LED RGB",
  caption1Text: "Ogni tasto ha il suo LED, del colore del suo suono, acceso mentre suona.",
  caption2Title: "Stampato in due pezzi",
  caption2Text: "Un guscio nero opaco con il logo stampato a filo in bianco, un fondo con le nervature, sei viti M3.",
  caption3Title: "Un display e una manopola",
  caption3Text:
    "La pagina e il suono in riproduzione a colpo d'occhio. Gira la manopola per cambiare pagina, tienila premuta per vedere tutti i nomi.",
  sounds:
    "Airhorn|Applausi|Rullo di tamburi|Ba dum tss|Trombone triste|Bruh|Boing|Laser|Ding|Wow|Clacson|Grilli|Vittoria|Fail|Bip|Disco graffiato|Risata malvagia|Ka-ching",
  stat1: "pad per pagina",
  stat2: "tasti funzione",
  stat3: "LED RGB",
  stat4: "cavo USB",
  featuresEyebrow: "01 · Cosa fa",
  featuresTitle: "Ogni suono a un tasto di distanza",
  feature1Title: "Le soundboard di Voicemod",
  feature1Text: "Le tue soundboard e le voci arrivano sui pad a pagine da 36, con le loro immagini e i loro colori.",
  feature2Title: "Le tue pagine",
  feature2Text: "Suoni, voci e azioni nell'ordine che vuoi, con i tuoi nomi e i colori dei LED.",
  feature3Title: "Macro",
  feature3Text: "Un pad, una linea del tempo intera: suoni uno sopra l'altro, voci e interruttori al momento giusto.",
  macroSteps: "Airhorn|attendi 0,5 s|Voce robot",
  feature4Title: "Sempre aggiornato",
  feature4Text: "L'app aggiorna sé stessa e il firmware del Drezimo, poi ti mostra le novità, nella tua lingua.",
  updateApp: "App",
  updateFirmware: "Firmware",
  updateDone: "Aggiornato",
  macroEyebrow: "02 · Macro",
  macroTitle: "Una gag intera, su un tasto",
  macroLead:
    "Metti i suoni su tracce lungo una linea del tempo, tagliali, sfumali, e aggiungi voci e interruttori al momento giusto. Una pressione sul Drezimo suona tutto, i suoni uno sopra l'altro. Prova questa.",
  macroName: "Entrata in scena",
  macroPadLabel: "Pad 1",
  macroPlay: "Suona",
  macroStop: "Ferma",
  macroLength: "Dura {value}",
  macroSeconds: "{value} s",
  macroTracks: "Tracce",
  macroTrack: "Traccia {n}",
  macroActions: "Azioni",
  macroFade: "Entra ed esce",
  macroOverItself: "Di nuovo, sopra sé stesso",
  macroVoiceOn: "Voce robot attiva",
  actionVoice: "Voce: Robot",
  actionBackground: "Effetti di sfondo: attivi",
  actionBack: "Torna alla voce",
  macroPadText: "Il suo pad respira mentre suona: premilo di nuovo per fermarla.",
  macroPoint1Title: "Suoni uno sopra l'altro",
  macroPoint1Text:
    "Tracce che suonano insieme, anche lo stesso suono due volte. Le opzioni di Voicemod che fermano, silenziano o ripetono vengono messe da parte per la macro e rimesse dopo.",
  macroPoint2Title: "Tagli e sfumature",
  macroPoint2Text: "Taglia una clip dove vuoi e dai forma al volume: entra, esce, o con punti tuoi.",
  macroPoint3Title: "Azioni al momento giusto",
  macroPoint3Text:
    "Voci, microfono, cambio voce ed effetti di sfondo, ognuno al suo momento. Voce e interruttori tornano com'erano alla fine, se vuoi.",
  macroPoint4Title: "Si modifica come una traccia",
  macroPoint4Text: "Annulla e ripeti, aggancio, più clip insieme, spostate con le frecce.",
  macroKeys: "Ctrl+Z|Ctrl+Y|Maiusc+clic|← →|Canc",
  feature5Title: "La ruota delle azioni",
  feature5Text:
    "Un gesto della manopola apre sullo schermo una ruota con fino a otto scorciatoie: pagine, azioni, macro. Giri per scegliere. Doppio tocco e pressione lunga durano quanto imposti tu.",
  feature6Title: "Pad che respirano",
  feature6Text: "Ogni pad delle tue pagine, e ogni macro, fisso, che respira o che lampeggia a riposo, in 14 colori.",
  feature7Title: "Il display, sul tuo schermo",
  feature7Text: "Il display del Drezimo accanto all'overlay: sempre, o con le azioni che scegli.",
  app4: "L'editor delle macro: tracce, tagli, sfumature e azioni",
  app5: "La ruota delle azioni, attorno al logo del Drezimo",
  appEyebrow: "03 · L'app",
  appTitle: "Tutto, in tempo reale, sullo schermo",
  app1: "La pagina del Drezimo in tempo reale, con le immagini di Voicemod",
  app2: "Le tue pagine, nell'ordine che vuoi",
  app3: "L'overlay, sopra ogni finestra",
  installEyebrow: "04 · Installazione",
  installTitle: "Pronto in un minuto",
  step1Title: "Scarica l'installer",
  step1Text: "Dal pulsante in alto. Sullo stesso PC serve Voicemod, gratuito o PRO.",
  step2Title: "Confermalo una volta",
  step2Text: "L'installer non è firmato, quindi Windows chiede: Ulteriori informazioni, poi Esegui comunque.",
  step3Title: "Collega il Drezimo",
  step3Text: "Un solo cavo USB. L'app parte con Windows e lo trova da sola.",
  encoreTitle: "Tocca a te.",
  encoreText: "Sei pad per scaldarti. Il Drezimo vero ne ha 36.",
  encorePad: "Suona {name}",
  soundsCredit: "Suoni: Freesound, CC0",
  footer: "Fatto a mano: elettronica, firmware, case e app.",
  source: "Il codice su GitHub",
  oledPage: "PAGINA",
  oledReady: "PRONTO",
  oledStop: "FERMATO",
  oledMute: "MIC MUTO",
  oledVoice: "VOCE ATTIVA",
  oledHear: "MI SENTO",
};

export function pickLanguage(languages: readonly string[]): Language {
  for (const language of languages) {
    const code = language.toLowerCase();
    if (code.startsWith("it")) return "it";
    if (code.startsWith("en")) return "en";
  }
  return "en";
}

const STORED = "drezimo-language";

export function isLanguage(value: string | null | undefined): value is Language {
  return value === "en" || value === "it";
}

/** A link's ?lang= first, then the one picked here before, then the browser's */
function startLanguage(): Language {
  if (typeof location === "undefined") return "en";
  const linked = new URLSearchParams(location.search).get("lang");
  if (isLanguage(linked)) return linked;
  try {
    const stored = localStorage.getItem(STORED);
    if (isLanguage(stored)) return stored;
  } catch {
    // Storage blocked: the browser's language
  }
  return pickLanguage(navigator.languages);
}

let current: Language = startLanguage();
const listeners: (() => void)[] = [];

export function language(): Language {
  return current;
}

/** What is drawn from the texts in code draws again */
export function onLanguage(listener: () => void) {
  listeners.push(listener);
}

/** Picked on the page: kept for the next visit, and in the address, so a link opens in it */
export function setLanguage(next: Language) {
  if (next === current) return;
  current = next;
  try {
    localStorage.setItem(STORED, next);
  } catch {
    // Not kept: the address still says it
  }
  const url = new URL(location.href);
  url.searchParams.set("lang", next);
  history.replaceState(history.state, "", url);
  applyTexts();
  for (const listener of listeners) listener();
}

export function t(key: Key, values: Record<string, string> = {}): string {
  const texts = current === "it" ? it : en;
  return texts[key].replace(/\{(\w+)\}/g, (_, name: string) => values[name] ?? "");
}

/** Every text the markup holds: content, labels and the pictures' descriptions, and the page's own language */
export function applyTexts(root: ParentNode = document) {
  for (const element of root.querySelectorAll<HTMLElement>("[data-t]")) {
    element.textContent = t(element.dataset.t as Key);
  }
  for (const element of root.querySelectorAll<HTMLElement>("[data-t-label]")) {
    element.setAttribute("aria-label", t(element.dataset.tLabel as Key));
  }
  for (const element of root.querySelectorAll<HTMLImageElement>("[data-t-alt]")) {
    element.alt = t(element.dataset.tAlt as Key);
  }
  document.documentElement.lang = current;
  document.querySelector("meta[name=description]")?.setAttribute("content", t("metaDescription"));
  for (const button of document.querySelectorAll<HTMLButtonElement>("[data-language]")) {
    button.setAttribute("aria-pressed", String(button.dataset.language === current));
  }
}
