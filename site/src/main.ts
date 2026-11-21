import "./styles.css";
import { loadDownloads } from "./download";
import { applyTexts, language, t } from "./i18n";
import type { Hero } from "./scene";

document.documentElement.lang = language;
applyTexts();

const touch = matchMedia("(pointer: coarse)").matches;
const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;
document.getElementById("hint")!.textContent = t(touch ? "hintTouch" : "hintPointer");

loadDownloads().then(({ version, exe, dmg }) => {
  for (const id of ["download", "download-again"]) (document.getElementById(id) as HTMLAnchorElement).href = exe;
  (document.getElementById("mac") as HTMLAnchorElement).href = dmg;
  document.getElementById("version")!.textContent = version ? t("version", { version }) : t("versionUnknown");
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
