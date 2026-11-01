import { invoke } from "@tauri-apps/api/core";

let sent = false;

/**
 * The window opens hidden and appears once its first complete page is built (`window_ready` in
 * src-tauri/src/window.rs), so it never shows an empty or half-built page. No requestAnimationFrame here: a hidden
 * window runs none.
 */
export function windowReady(): void {
  if (sent) return;
  sent = true;
  invoke("window_ready");
}
