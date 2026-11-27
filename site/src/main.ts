import "@fontsource-variable/bricolage-grotesque";
import "./styles.css";
import { loadDownloads } from "./download";
import { applyTexts, isLanguage, onLanguage, setLanguage, t } from "./i18n";
import { startPage } from "./page";
import { prefetchSounds } from "./sound";
import type { Hero } from "./scene";

applyTexts();
for (const button of document.querySelectorAll<HTMLButtonElement>("[data-language]")) {
  button.addEventListener("click", () => {
    if (isLanguage(button.dataset.language)) setLanguage(button.dataset.language);
  });
}

const touch = matchMedia("(pointer: coarse)").matches;
const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;
const hint = () => (document.getElementById("hint")!.textContent = t(touch ? "hintTouch" : "hintPointer"));
hint();
onLanguage(hint);
startPage(reducedMotion);
prefetchSounds();

loadDownloads().then(({ version, exe, dmg }) => {
  for (const id of ["download", "download-again"]) (document.getElementById(id) as HTMLAnchorElement).href = exe;
  for (const id of ["mac", "mac-again"]) (document.getElementById(id) as HTMLAnchorElement).href = dmg;
  if (!version) return;
  const released = document.getElementById("released")!;
  const say = () => (released.querySelector("span")!.textContent = t("released", { version }));
  say();
  onLanguage(say);
  released.hidden = false;
});

const hero = document.getElementById("hero")!;
const intro = document.querySelector<HTMLElement>(".intro")!;
const canvas = document.getElementById("scene") as HTMLCanvasElement;
const captions = [...document.querySelectorAll<HTMLElement>(".caption")];

const showPicture = () => document.body.classList.add("no-webgl");
let scene: Hero | null = null;
import("./scene")
  .then(({ startHero }) => {
    scene = startHero(canvas, reducedMotion, () => document.body.classList.add("ready"), showPicture);
    onScroll();
  })
  .catch(showPicture);

function onScroll() {
  const box = hero.getBoundingClientRect();
  const progress = Math.min(1, Math.max(0, -box.top / (box.height - innerHeight)));
  scene?.setProgress(progress);
  hero.style.setProperty("--progress", progress.toFixed(4));
  intro.inert = progress > 0.12;
  for (const caption of captions) {
    const shown = progress >= Number(caption.dataset.from) && progress < Number(caption.dataset.to);
    caption.classList.toggle("shown", shown);
  }
}
addEventListener("scroll", onScroll, { passive: true });
addEventListener("resize", onScroll);
onScroll();

new IntersectionObserver((entries) => scene?.setVisible(entries[entries.length - 1].isIntersecting)).observe(canvas);
