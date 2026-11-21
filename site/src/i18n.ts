export type Language = "en" | "it";

const en = {
  title: "Drezimo Soundboard",
  tagline: "36 pads, 6 function keys, a knob and a display. Voicemod at your fingertips.",
  download: "Download for Windows",
  version: "Version {version} · needs Voicemod",
  versionUnknown: "Needs Voicemod, free or PRO",
  mac: "Also for macOS (Apple Silicon)",
  hintPointer: "Click the keys · scroll to open it",
  hintTouch: "Tap the keys · scroll to open it",
  caption1Title: "42 keys, 42 RGB LEDs",
  caption1Text: "Every key has its own LED, in the colour of its sound, lit while it plays.",
  caption2Title: "Printed in two parts",
  caption2Text: "A matte black shell with the logo printed flush in white, a ribbed bottom, six M3 screws.",
  caption3Title: "A display and a knob",
  caption3Text: "The page and the sound playing at a glance. Turn the knob to change page, hold it to see every name.",
  featuresTitle: "What it does",
  feature1Title: "Voicemod's soundboards",
  feature1Text: "Your soundboards and voices arrive on the pads as pages of 36, with their pictures and colours.",
  feature2Title: "Your own pages",
  feature2Text: "Sounds, voices and actions in the order you want, with your own names and LED colours.",
  feature3Title: "Macros",
  feature3Text: "One pad, a whole sequence: sounds, waits, voices and switches.",
  feature4Title: "Always up to date",
  feature4Text: "The app updates itself and the Drezimo's firmware, and tells you what is new.",
  appTitle: "The app",
  app1: "The Drezimo's page, live, with Voicemod's pictures",
  app2: "Your own pages, in the order you want",
  app3: "The overlay, over every window",
  installTitle: "Ready in a minute",
  step1Title: "Download the installer",
  step1Text: "From the button at the top. Voicemod, free or PRO, must be on the same PC.",
  step2Title: "Confirm it once",
  step2Text: "The installer is not signed, so Windows asks: More info, then Run anyway.",
  step3Title: "Plug in the Drezimo",
  step3Text: "One USB cable. The app starts with Windows and finds it on its own.",
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
  title: "Drezimo Soundboard",
  tagline: "36 pad, 6 tasti funzione, una manopola e un display. Voicemod sotto le dita.",
  download: "Scarica per Windows",
  version: "Versione {version} · serve Voicemod",
  versionUnknown: "Serve Voicemod, gratuito o PRO",
  mac: "Anche per macOS (Apple Silicon)",
  hintPointer: "Clicca i tasti · scorri per aprirlo",
  hintTouch: "Tocca i tasti · scorri per aprirlo",
  caption1Title: "42 tasti, 42 LED RGB",
  caption1Text: "Ogni tasto ha il suo LED, del colore del suo suono, acceso mentre suona.",
  caption2Title: "Stampato in due pezzi",
  caption2Text: "Un guscio nero opaco con il logo stampato a filo in bianco, un fondo con le nervature, sei viti M3.",
  caption3Title: "Un display e una manopola",
  caption3Text:
    "La pagina e il suono in riproduzione a colpo d'occhio. Gira la manopola per cambiare pagina, tienila premuta per vedere tutti i nomi.",
  featuresTitle: "Cosa fa",
  feature1Title: "Le soundboard di Voicemod",
  feature1Text: "Le tue soundboard e le voci arrivano sui pad a pagine da 36, con le loro immagini e i loro colori.",
  feature2Title: "Le tue pagine",
  feature2Text: "Suoni, voci e azioni nell'ordine che vuoi, con i tuoi nomi e i colori dei LED.",
  feature3Title: "Macro",
  feature3Text: "Un pad, una sequenza intera: suoni, attese, voci e interruttori.",
  feature4Title: "Sempre aggiornato",
  feature4Text: "L'app aggiorna se stessa e il firmware del Drezimo, e ti dice cosa c'è di nuovo.",
  appTitle: "L'app",
  app1: "La pagina del Drezimo, dal vivo, con le immagini di Voicemod",
  app2: "Le tue pagine, nell'ordine che vuoi",
  app3: "L'overlay, sopra ogni finestra",
  installTitle: "Pronto in un minuto",
  step1Title: "Scarica l'installer",
  step1Text: "Dal pulsante in alto. Sullo stesso PC serve Voicemod, gratuito o PRO.",
  step2Title: "Confermalo una volta",
  step2Text: "L'installer non è firmato, quindi Windows chiede: Ulteriori informazioni, poi Esegui comunque.",
  step3Title: "Collega il Drezimo",
  step3Text: "Un solo cavo USB. L'app parte con Windows e lo trova da sola.",
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

export const language = pickLanguage(typeof navigator === "undefined" ? [] : navigator.languages);
const texts = language === "it" ? it : en;

export function t(key: Key, values: Record<string, string> = {}): string {
  return texts[key].replace(/\{(\w+)\}/g, (_, name: string) => values[name] ?? "");
}

export function applyTexts(root: ParentNode = document) {
  for (const element of root.querySelectorAll<HTMLElement>("[data-t]")) {
    element.textContent = t(element.dataset.t as Key);
  }
}
