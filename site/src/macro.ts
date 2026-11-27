import { type Key, onLanguage, t } from "./i18n";
import { type Clip, type Playing, playTimeline, SOUNDS } from "./sound";

/** What the macro shows and plays: the app's time line, with the page's own sounds */
interface Drawn extends Clip {
  track: number;
  /** Its length in seconds, as drawn: the sound's own, or its cut */
  length: number;
  note?: Key;
}

const CLIPS: Drawn[] = [
  { sound: "drum-roll", track: 0, at: 0, length: 1.71 },
  { sound: "rimshot", track: 0, at: 1.75, length: 1.07 },
  { sound: "applause", track: 0, at: 3, length: 3.56, fade: true, note: "macroFade" },
  { sound: "airhorn", track: 1, at: 1.8, length: 1.61 },
  { sound: "victory", track: 1, at: 3.6, length: 1.7 },
  { sound: "airhorn", track: 2, at: 2.3, length: 1.61, note: "macroOverItself" },
];

const ACTIONS: { at: number; text: Key; colour: string }[] = [
  { at: 0, text: "actionVoice", colour: "#b45cff" },
  { at: 1.8, text: "actionBackground", colour: "#22c8e0" },
  { at: 6.2, text: "actionBack", colour: "#b45cff" },
];

const TRACKS = 3;
const SHOWN = 7;
const END = Math.max(...CLIPS.map((clip) => clip.at + clip.length));

const hue = (sound: number) => (sound * 47 + 30) % 360;
const percent = (seconds: number) => `${(seconds / SHOWN) * 100}%`;
const seconds = (value: number) => t("macroSeconds", { value: String(Math.round(value * 10) / 10) });

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text = "") {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

function ruler() {
  const row = element("div", "ruler");
  row.append(element("span", "head", t("macroTracks")));
  const marks = element("div", "marks");
  for (let second = 0; second < SHOWN; second++) {
    const mark = element("span", "", `${second}s`);
    mark.style.left = percent(second);
    marks.append(mark);
  }
  row.append(marks);
  return row;
}

/** The volume's curve over a clip that fades in and out, as the app draws it */
function curve() {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 100 20");
  svg.setAttribute("preserveAspectRatio", "none");
  svg.setAttribute("class", "curve");
  const line = document.createElementNS("http://www.w3.org/2000/svg", "polyline");
  line.setAttribute("points", "0,19 30,2 70,2 100,19");
  svg.append(line);
  return svg;
}

function lane(track: number, names: string[]) {
  const row = element("div", "lane");
  row.append(element("span", "head", t("macroTrack", { n: String(track + 1) })));
  const body = element("div", "body");
  for (const clip of CLIPS.filter((item) => item.track === track)) {
    const index = SOUNDS.indexOf(clip.sound);
    const node = element("div", "clip");
    node.style.left = percent(clip.at);
    node.style.width = percent(clip.length);
    node.style.setProperty("--h", String(hue(index)));
    node.dataset.at = String(clip.at);
    node.dataset.end = String(clip.at + clip.length);
    const title = element("span", "name", names[index]);
    title.append(element("small", "", seconds(clip.length)));
    node.append(title);
    if (clip.fade) node.append(curve());
    if (clip.note) node.append(element("span", "note", t(clip.note)));
    body.append(node);
  }
  row.append(body);
  return row;
}

function actions() {
  const row = element("div", "lane action-lane");
  row.append(element("span", "head", t("macroActions")));
  const body = element("div", "body");
  for (const action of ACTIONS) {
    const node = element("span", "action", t(action.text));
    node.style.left = percent(action.at);
    node.style.setProperty("--c", action.colour);
    node.dataset.at = String(action.at);
    body.append(node);
  }
  row.append(body);
  return row;
}

/** The Macro section's editor: press Play, or its pad, and the six clips play over each other as on a Drezimo */
export function buildMacroEditor() {
  const editor = document.getElementById("macro-editor");
  if (!editor) return;
  const timeline = editor.querySelector<HTMLElement>(".timeline")!;
  const button = editor.querySelector<HTMLButtonElement>("#macro-play")!;
  const label = button.querySelector("span")!;
  const pad = editor.querySelector<HTMLButtonElement>("#macro-pad")!;
  const voice = editor.querySelector<HTMLElement>("#macro-voice")!;
  const scroller = editor.querySelector<HTMLElement>(".scroller")!;
  const head = element("div", "playhead");
  let clips: HTMLElement[] = [];
  let marks: HTMLElement[] = [];
  let playing: Playing | null = null;
  let frame = 0;

  // In the page's language, again when it changes: what plays goes on
  const fill = () => {
    const names = t("sounds").split("|");
    voice.textContent = t("macroVoiceOn");
    editor.querySelector("#macro-length")!.textContent = t("macroLength", { value: seconds(END) });
    const lanes = Array.from({ length: TRACKS }, (_, track) => lane(track, names));
    timeline.replaceChildren(ruler(), ...lanes, actions(), head);
    clips = [...timeline.querySelectorAll<HTMLElement>(".clip")];
    marks = [...timeline.querySelectorAll<HTMLElement>(".action")];
    label.textContent = t(playing ? "macroStop" : "macroPlay");
  };
  fill();
  onLanguage(fill);

  const draw = (at: number | null) => {
    editor.classList.toggle("running", at !== null);
    head.style.setProperty("--at", at === null ? "0" : String(Math.min(at, SHOWN) / SHOWN));
    for (const clip of clips) {
      clip.classList.toggle("on", at !== null && at >= Number(clip.dataset.at) && at < Number(clip.dataset.end));
    }
    for (const mark of marks) mark.classList.toggle("passed", at !== null && at >= Number(mark.dataset.at));
    // The robot voice from the first action until the last one puts the user's back
    voice.hidden = at === null || at >= ACTIONS[ACTIONS.length - 1].at;
    label.textContent = t(at === null ? "macroPlay" : "macroStop");
  };

  const stop = () => {
    playing?.stop();
    playing = null;
    cancelAnimationFrame(frame);
    draw(null);
  };

  const tick = () => {
    if (!playing) return;
    const at = Math.max(0, playing.elapsed());
    if (at > END + 0.2) return stop();
    draw(at);
    // On a narrow screen the time line follows the playhead
    if (scroller.scrollWidth > scroller.clientWidth) {
      scroller.scrollLeft = head.offsetLeft - scroller.clientWidth * 0.4;
    }
    frame = requestAnimationFrame(tick);
  };

  // Pressed again while it runs, it stops, as the app's macros do by default
  const press = () => {
    if (playing) return stop();
    playing = playTimeline(CLIPS);
    if (playing) frame = requestAnimationFrame(tick);
  };

  button.addEventListener("click", press);
  pad.addEventListener("click", press);
  draw(null);
}
