import {
  ACESFilmicToneMapping,
  CanvasTexture,
  Color,
  DirectionalLight,
  Group,
  HalfFloatType,
  MathUtils,
  Mesh,
  MeshBasicMaterial,
  MeshStandardMaterial,
  Object3D,
  PerspectiveCamera,
  PlaneGeometry,
  PMREMGenerator,
  Raycaster,
  Scene,
  SRGBColorSpace,
  Vector2,
  WebGLRenderer,
  WebGLRenderTarget,
} from "three";
import { RoomEnvironment } from "three/addons/environments/RoomEnvironment.js";
import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";
import { MeshoptDecoder } from "three/addons/libs/meshopt_decoder.module.js";
import { EffectComposer } from "three/addons/postprocessing/EffectComposer.js";
import { OutputPass } from "three/addons/postprocessing/OutputPass.js";
import { RenderPass } from "three/addons/postprocessing/RenderPass.js";
import { UnrealBloomPass } from "three/addons/postprocessing/UnrealBloomPass.js";
import { buildBoard, layout, type Key } from "./board";
import { t } from "./i18n";
import { explode, PAGE_HUES, pressDepth, waveColour, type Rgb } from "./motion";
import { playKey } from "./sound";

const BACKGROUND = 0x0e0e10;
const PAGES = PAGE_HUES.length;
const TRAVEL = layout.keycap.restBottom - layout.keycap.pressedBottom;
const FN = { prev: 37, next: 38, stop: 39, mute: 40, voice: 41, hear: 42 };
const FLASH_MS = 650;
const INTRO_MS = 1600;
// The letters on the pads' keycaps, row by row: typing one plays its pad.
const TYPED = "qwertyasdfghzxcvbnuiopjk123456\\/'.,";

const ease = (x: number) => 1 - (1 - MathUtils.clamp(x, 0, 1)) ** 3;

export interface Hero {
  setProgress(progress: number): void;
  setVisible(visible: boolean): void;
}

export function startHero(canvas: HTMLCanvasElement, reducedMotion: boolean, onReady: () => void, onFailed: () => void): Hero {
  const renderer = new WebGLRenderer({ canvas });
  canvas.addEventListener("webglcontextlost", onFailed);
  renderer.setPixelRatio(Math.min(devicePixelRatio, 1.75));
  renderer.toneMapping = ACESFilmicToneMapping;
  renderer.toneMappingExposure = 0.95;
  renderer.outputColorSpace = SRGBColorSpace;

  const scene = new Scene();
  scene.background = new Color(BACKGROUND);
  const pmrem = new PMREMGenerator(renderer);
  scene.environment = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
  scene.environmentIntensity = 0.55;

  const key = new DirectionalLight(0xffffff, 1.1);
  key.position.set(-120, 260, 180);
  const rim = new DirectionalLight(0x9fb4ff, 1.4);
  rim.position.set(160, 120, -220);
  scene.add(key, rim);

  const camera = new PerspectiveCamera(26, 1, 10, 3000);
  // The bloom draws into its own buffer: the smoothing of edges has to be asked of that one.
  const composer = new EffectComposer(renderer, new WebGLRenderTarget(1, 1, { type: HalfFloatType, samples: 4 }));
  composer.addPass(new RenderPass(scene, camera));
  const bloom = new UnrealBloomPass(new Vector2(256, 256), 0.8, 0.45, 1.35);
  composer.addPass(bloom);
  composer.addPass(new OutputPass());

  const device = new Group();
  const tilt = new Group();
  tilt.add(device);
  scene.add(tilt);

  const { board, caps, keys, knob, oled } = buildBoard();
  const shell = new Group();
  const bottom = new Group();
  device.add(shell, board, caps, bottom);
  device.visible = false;

  const shadow = new Mesh(
    new PlaneGeometry(260, 320).rotateX(-Math.PI / 2),
    new MeshBasicMaterial({ map: shadowTexture(), transparent: true, depthWrite: false })
  );
  shadow.position.y = layout.lowest - 34;
  tilt.add(shadow);

  const loader = new GLTFLoader().setMeshoptDecoder(MeshoptDecoder);
  const body = new MeshStandardMaterial({ color: 0x121214, roughness: 0.8, metalness: 0.02 });
  const inlay = new MeshStandardMaterial({ color: 0xf1eee8, roughness: 0.6 });
  loader.load(
    `${import.meta.env.BASE_URL}drezimo.glb`,
    (gltf) => {
      const parts: Record<string, Object3D> = {};
      gltf.scene.traverse((node) => (parts[node.name] = node));
      for (const [name, target, material] of [
        ["shell_body", shell, body],
        ["shell_inlay", shell, inlay],
        ["bottom", bottom, body],
      ] as const) {
        const mesh = parts[name] as Mesh;
        mesh.material = material;
        target.add(mesh);
      }
      device.visible = true;
      loaded = true;
      startedAt = performance.now();
      onReady();
    },
    undefined,
    onFailed
  );

  // What the board is doing, as the page shows it.
  let loaded = false;
  let page = 0;
  let line = t("oledReady");
  let level = 0;
  let stoppedAt = -Infinity;
  const toggles = { [FN.mute]: false, [FN.voice]: false, [FN.hear]: false };
  const toggleColour: Record<number, Rgb> = {
    [FN.mute]: [1, 0.05, 0.05],
    [FN.voice]: [0.7, 0.2, 1],
    [FN.hear]: [0.1, 1, 0.35],
  };
  let knobTurn = 0;
  let knobAngle = 0;

  function press(n: number, now: number) {
    const pressed = keys[n - 1];
    pressed.pressedAt = now;
    playKey(n, page);
    level = 1;
    if (n <= 36) line = `PAD ${n}`;
    else if (n === FN.prev || n === FN.next) turnPage(n === FN.next ? 1 : -1);
    else if (n === FN.stop) {
      stoppedAt = now;
      line = t("oledStop");
      level = 0;
    } else {
      toggles[n] = !toggles[n];
      const label = { [FN.mute]: "oledMute", [FN.voice]: "oledVoice", [FN.hear]: "oledHear" } as const;
      line = toggles[n] ? t(label[n]) : t("oledReady");
    }
  }

  function turnPage(by: number) {
    page = (page + by + PAGES) % PAGES;
    knobTurn += by * (Math.PI / 10);
    line = `${t("oledPage")} ${page + 1}`;
  }

  // Pointer: parallax, drag to turn, click or tap a key.
  const pointer = new Vector2();
  const raycaster = new Raycaster();
  let lookX = 0;
  let lookY = 0;
  let dragYaw = 0;
  let drag: { x: number; y: number; yaw: number; moved: boolean } | null = null;

  addEventListener("pointermove", (event) => {
    if (!reducedMotion) {
      lookX = (event.clientX / innerWidth) * 2 - 1;
      lookY = (event.clientY / innerHeight) * 2 - 1;
    }
    if (drag) {
      const dx = event.clientX - drag.x;
      if (Math.abs(dx) > 6 || Math.abs(event.clientY - drag.y) > 6) drag.moved = true;
      dragYaw = drag.yaw + dx * 0.008;
    }
  });
  canvas.addEventListener("pointerdown", (event) => {
    if (event.button !== 0) return;
    drag = { x: event.clientX, y: event.clientY, yaw: dragYaw, moved: false };
  });
  addEventListener("pointerup", (event) => {
    const wasDrag = drag?.moved;
    drag = null;
    if (wasDrag || !loaded || event.button !== 0 || event.target !== canvas) return;
    const box = canvas.getBoundingClientRect();
    pointer.set(((event.clientX - box.left) / box.width) * 2 - 1, -((event.clientY - box.top) / box.height) * 2 + 1);
    raycaster.setFromCamera(pointer, camera);
    const hit = raycaster.intersectObjects(caps.children, true)[0]?.object;
    if (hit?.userData.key) press(hit.userData.key, performance.now());
    else if (hit?.userData.knob) {
      turnPage(1);
      playKey(FN.next, page);
    }
  });
  addEventListener("pointercancel", () => (drag = null));
  addEventListener("keydown", (event) => {
    if (event.repeat || event.metaKey || event.ctrlKey || event.altKey || !visible || !loaded) return;
    const typed = event.key === "Delete" ? 36 : TYPED.indexOf(event.key.toLowerCase()) + 1;
    const n = typed || ({ ArrowLeft: FN.prev, ArrowRight: FN.next, Escape: FN.stop } as Record<string, number>)[event.key];
    if (!n) return;
    press(n, performance.now());
  });

  function resize() {
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    renderer.setSize(width, height, false);
    composer.setSize(width, height);
    camera.aspect = width / height;
    const wide = width > 900;
    const portrait = width < height;
    const distance = portrait ? 640 + 420 * (height / width - 1) : 640;
    camera.position.set(0, distance * 0.78, distance * 0.62);
    camera.lookAt(0, 0, 0);
    if (wide) camera.setViewOffset(width, height, -width * 0.14, 0, width, height);
    else if (portrait) camera.setViewOffset(width, height, 0, -height * 0.2, width, height);
    else camera.clearViewOffset();
    camera.updateProjectionMatrix();
  }
  new ResizeObserver(resize).observe(canvas);
  resize();

  let progress = 0;
  let shownProgress = 0;
  let visible = true;
  let startedAt = Infinity;
  let last = performance.now();
  let yaw = -0.5;
  let drawn = "";

  function frame(now: number) {
    requestAnimationFrame(frame);
    const dt = Math.min(0.05, (now - last) / 1000);
    last = now;
    if (!visible) return;

    const seconds = now / 1000;
    const intro = reducedMotion ? 1 : ease((now - startedAt) / INTRO_MS);
    shownProgress += (progress - shownProgress) * (reducedMotion ? 1 : 1 - Math.exp(-dt * 8));
    const open = explode(shownProgress);

    const idle = reducedMotion ? 0 : 0.22 * Math.sin(seconds * 0.35);
    const target = idle + dragYaw + lookX * 0.18 + open.spin;
    yaw += (target - yaw) * (1 - Math.exp(-dt * 4));
    device.rotation.y = yaw - 0.5 * (1 - intro);
    tilt.rotation.x = open.tilt + lookY * 0.06 - 0.25 * (1 - intro);
    device.position.y = -30 * (1 - intro);

    shell.position.y = open.shell;
    bottom.position.y = open.bottom;
    caps.position.y = open.keys;
    shadow.material.opacity = 0.85 - 0.4 * shownProgress;

    knobAngle += (knobTurn - knobAngle) * (1 - Math.exp(-dt * 12));
    knob.rotation.y = -knobAngle;

    const sinceStop = (now - stoppedAt) / 900;
    const dim = sinceStop < 1 ? sinceStop : 1;
    for (const k of keys) {
      const since = now - k.pressedAt;
      const dropIn = reducedMotion ? 0 : Math.max(0, 1 - ease((now - startedAt - 500 - (k.x - k.z + 140) * 2.2) / 450));
      k.cap.position.y = k.restY - TRAVEL * pressDepth(since) + 60 * dropIn;
      lightKey(k, since, seconds, dim * (1 - dropIn));
    }

    level = Math.max(0, level - dt * 0.9);
    const showing = `${page}|${line}|${Math.round(level * 100)}`;
    if (showing !== drawn) {
      drawn = showing;
      oled.draw({ page, pages: PAGES, line, level }, t("oledPage"));
    }
    composer.render();
  }

  const scratch = new Color();
  function lightKey(k: Key, since: number, seconds: number, strength: number) {
    let colour: Rgb;
    if (k.n > 36) {
      colour = toggles[k.n] ? toggleColour[k.n] : [0.5, 0.33, 0.06];
    } else {
      colour = reducedMotion ? waveColour(k, 0, page) : waveColour(k, seconds, page);
    }
    const flash = since < FLASH_MS ? 1 - since / FLASH_MS : 0;
    const boost = 1 + 2.2 * flash;
    scratch.setRGB(
      (colour[0] + flash * 0.6) * boost * strength,
      (colour[1] + flash * 0.6) * boost * strength,
      (colour[2] + flash * 0.6) * boost * strength
    );
    k.glow.color.copy(scratch).multiplyScalar(2.6);
    k.capMaterial.emissive.copy(scratch).multiplyScalar(1.1);
  }

  requestAnimationFrame(frame);

  return {
    setProgress(value) {
      progress = value;
    },
    setVisible(value) {
      visible = value;
    },
  };
}

function shadowTexture() {
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = 256;
  const g = canvas.getContext("2d")!;
  const gradient = g.createRadialGradient(128, 128, 10, 128, 128, 128);
  gradient.addColorStop(0, "rgba(0,0,0,0.9)");
  gradient.addColorStop(0.55, "rgba(0,0,0,0.45)");
  gradient.addColorStop(1, "rgba(0,0,0,0)");
  g.fillStyle = gradient;
  g.fillRect(0, 0, 256, 256);
  return new CanvasTexture(canvas);
}
