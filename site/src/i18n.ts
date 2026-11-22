export type Language = "en" | "it";

const en = {
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
  feature3Text: "One pad, a whole sequence: sounds, waits, voices and switches.",
  macroSteps: "Airhorn|wait 0.5 s|Robot voice",
  feature4Title: "Always up to date",
  feature4Text: "The app updates itself and the Drezimo's firmware, and tells you what is new.",
  updateApp: "App",
  updateFirmware: "Firmware",
  updateDone: "Up to date",
  appEyebrow: "02 · The app",
  appTitle: "Everything, live, on your screen",
  app1: "The Drezimo's page, live, with Voicemod's pictures",
  app2: "Your own pages, in the order you want",
  app3: "The overlay, over every window",
  installEyebrow: "03 · Setup",
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
  feature3Text: "Un pad, una sequenza intera: suoni, attese, voci e interruttori.",
  macroSteps: "Airhorn|attendi 0,5 s|Voce robot",
  feature4Title: "Sempre aggiornato",
  feature4Text: "L'app aggiorna sé stessa e il firmware del Drezimo, e ti dice cosa c'è di nuovo.",
  updateApp: "App",
  updateFirmware: "Firmware",
  updateDone: "Aggiornato",
  appEyebrow: "02 · L'app",
  appTitle: "Tutto, in tempo reale, sullo schermo",
  app1: "La pagina del Drezimo in tempo reale, con le immagini di Voicemod",
  app2: "Le tue pagine, nell'ordine che vuoi",
  app3: "L'overlay, sopra ogni finestra",
  installEyebrow: "03 · Installazione",
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
