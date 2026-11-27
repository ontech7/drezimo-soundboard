import { onLanguage, t } from "./i18n";
import { buildMacroEditor } from "./macro";
import { playSound } from "./sound";

const hue = (i: number) => (i * 47 + 30) % 360;

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text = "") {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

/** Lights a pad-like button for a moment, the way the Drezimo's LED does. */
function flash(node: HTMLElement) {
  node.classList.remove("hit");
  void node.offsetWidth;
  node.classList.add("hit");
}

/** Two rows of sound names sliding past each other; a click on one plays it. Decoration for screen readers. */
function buildTicker(reducedMotion: boolean) {
  const ticker = document.getElementById("ticker")!;
  ticker.replaceChildren();
  ticker.setAttribute("aria-hidden", "true");
  const names = t("sounds").split("|");
  const half = Math.ceil(names.length / 2);
  [names.slice(0, half), names.slice(half)].forEach((row, r) => {
    const track = element("div", `track${r ? " reverse" : ""}`);
    // Half the track scrolls by in a loop, so each half must be wider than the widest screen.
    const copies = reducedMotion ? 1 : 8;
    for (let copy = 0; copy < copies; copy++) {
      row.forEach((name, i) => {
        const index = r * half + i;
        const chip = element("button", "chip", name);
        chip.type = "button";
        chip.tabIndex = -1;
        chip.style.setProperty("--h", String(hue(index)));
        chip.addEventListener("click", () => {
          playSound(index);
          flash(chip);
        });
        track.append(chip);
      });
    }
    ticker.append(track);
  });
}

function buildMiniPads() {
  const grid = document.getElementById("mini-pads")!;
  for (let i = 0; i < 36; i++) {
    const pad = element("i");
    pad.style.setProperty("--d", String((i % 6) + Math.floor(i / 6)));
    grid.append(pad);
  }
}

function buildMacro() {
  const macro = document.getElementById("mini-macro")!;
  macro.replaceChildren();
  t("macroSteps")
    .split("|")
    .forEach((step, i) => {
      const chip = element("span", "step", step);
      chip.style.setProperty("--i", String(i));
      macro.append(chip);
    });
}

/** The Macro editor's keys, as key caps */
function buildKeys() {
  const keys = document.getElementById("macro-keys")!;
  keys.replaceChildren();
  for (const key of t("macroKeys").split("|")) keys.append(element("kbd", "", key));
}

const ENCORE_SOUNDS = [0, 3, 6, 7, 12, 17];

/** The last section's six pads, each with one of the sounds from the ticker. */
function buildPads() {
  const pads = document.getElementById("pads")!;
  pads.replaceChildren();
  const names = t("sounds").split("|");
  ENCORE_SOUNDS.forEach((sound, column) => {
    const pad = element("button", "pad");
    pad.type = "button";
    pad.title = names[sound];
    pad.setAttribute("aria-label", t("encorePad", { name: names[sound] }));
    pad.style.setProperty("--h", String(hue(column * 3)));
    const play = () => {
      playSound(sound);
      flash(pad);
    };
    // A mouse plays on the way down, for no lag; touch waits for the tap, so a swipe to scroll stays silent.
    let playedOnPress = false;
    pad.addEventListener("pointerdown", (event) => {
      playedOnPress = event.pointerType !== "touch" && event.button === 0;
      if (playedOnPress) play();
    });
    pad.addEventListener("click", (event) => {
      if (event.detail === 0 || !playedOnPress) play();
      playedOnPress = false;
    });
    pads.append(pad);
  });
}

function countUp(node: HTMLElement, reducedMotion: boolean) {
  const target = Number(node.dataset.count);
  if (reducedMotion || target < 2) return;
  const started = performance.now();
  const step = (now: number) => {
    const x = Math.min(1, (now - started) / 1100);
    node.textContent = String(Math.round(target * (1 - (1 - x) ** 3)));
    if (x < 1) requestAnimationFrame(step);
  };
  requestAnimationFrame(step);
}

function reveal(reducedMotion: boolean) {
  const targets = document.querySelectorAll<HTMLElement>("[data-reveal]");
  const show = (node: HTMLElement) => {
    node.classList.add("in");
    for (const number of node.querySelectorAll<HTMLElement>("[data-count]")) countUp(number, reducedMotion);
  };
  if (reducedMotion || !("IntersectionObserver" in window)) {
    targets.forEach(show);
    return;
  }
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        show(entry.target as HTMLElement);
        observer.unobserve(entry.target);
      }
    },
    { rootMargin: "0px 0px -12% 0px" }
  );
  targets.forEach((node) => observer.observe(node));
}

/** Cards light up under the pointer, in their own colour. */
function spotlight() {
  for (const card of document.querySelectorAll<HTMLElement>(".card")) {
    card.addEventListener("pointermove", (event) => {
      const box = card.getBoundingClientRect();
      card.style.setProperty("--mx", `${event.clientX - box.left}px`);
      card.style.setProperty("--my", `${event.clientY - box.top}px`);
    });
  }
}

/** The app's screenshot lies back and straightens up as it scrolls in. */
function tiltShot(reducedMotion: boolean) {
  const shot = document.getElementById("app-shot")!;
  if (reducedMotion) {
    shot.style.setProperty("--lean", "0");
    return;
  }
  let queued = false;
  const update = () => {
    queued = false;
    const box = shot.getBoundingClientRect();
    if (box.bottom < 0 || box.top > innerHeight) return;
    const lean = Math.min(1, Math.max(0, (box.top - innerHeight * 0.15) / (innerHeight * 0.7)));
    shot.style.setProperty("--lean", lean.toFixed(3));
  };
  addEventListener(
    "scroll",
    () => {
      if (!queued) {
        queued = true;
        requestAnimationFrame(update);
      }
    },
    { passive: true }
  );
  update();
}

export function startPage(reducedMotion: boolean) {
  document.documentElement.classList.add("js");
  buildTicker(reducedMotion);
  buildMiniPads();
  buildMacro();
  buildMacroEditor();
  buildKeys();
  buildPads();
  onLanguage(() => {
    buildTicker(reducedMotion);
    buildMacro();
    buildKeys();
    buildPads();
  });
  reveal(reducedMotion);
  spotlight();
  tiltShot(reducedMotion);
}
