/**
 * Keeps the window an app rather than a web page: no browser menu on a right click, no reload, print, find, view
 * source or zoom from the keyboard, no file or link dropped in to open, no middle-click autoscroll. Text fields keep
 * their own menu, for cut, copy and paste.
 */
export function guardTheWindow(): void {
  window.addEventListener("contextmenu", (event) => {
    if (!editable(event.target)) event.preventDefault();
  });
  window.addEventListener(
    "keydown",
    (event) => {
      if (browserShortcut(event)) event.preventDefault();
    },
    { capture: true }
  );
  for (const type of ["dragover", "drop"] as const) {
    window.addEventListener(type, (event) => {
      if (!fromOutside(event)) return;
      event.preventDefault();
      if (event.dataTransfer) event.dataTransfer.dropEffect = "none";
    });
  }
  // Windows' autoscroll
  window.addEventListener("mousedown", (event) => {
    if (event.button === 1) event.preventDefault();
  });
}

function editable(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || target.matches("textarea, input:not([type=range], [type=checkbox], [type=radio])"))
  );
}

/** Reload, find and caret browsing on their own; with Ctrl or Cmd: reload, print, find, view source, save, open, zoom */
const ALONE = ["f5", "f3", "f7"];
const WITH_MODIFIER = ["r", "p", "f", "g", "u", "s", "o", "+", "-", "=", "0"];

function browserShortcut(event: KeyboardEvent): boolean {
  const key = event.key.toLowerCase();
  if (ALONE.includes(key)) return true;
  return (event.ctrlKey || event.metaKey) && !event.altKey && WITH_MODIFIER.includes(key);
}

/** A file or a link dragged in from another app: the page's own drags carry neither */
function fromOutside(event: DragEvent): boolean {
  const types = event.dataTransfer?.types ?? [];
  return types.includes("Files") || types.includes("text/uri-list");
}
