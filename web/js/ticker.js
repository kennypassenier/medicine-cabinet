// Keeps time-dependent screens honest: runs `fn` every minute while the tab
// is visible and again the moment the tab or window regains focus, so a
// "due" dose never shows stale.

/** Refresh period in milliseconds. */
export const REFRESH_MS = 60_000;

/** The event that asks every screen to refresh now (after an Undo that changed data on another screen). */
export const REFRESH_EVENT = "mc-refresh";

/** Ask the screen on show to reload its data. */
export const refreshNow = () => window.dispatchEvent(new Event(REFRESH_EVENT));

/**
 * @param {() => void | Promise<void>} fn
 * @returns {() => void} stop
 */
export function onTick(fn) {
  const run = () => {
    if (document.visibilityState === "visible") void fn();
  };
  const timer = setInterval(run, REFRESH_MS);
  document.addEventListener("visibilitychange", run);
  window.addEventListener("focus", run);
  window.addEventListener(REFRESH_EVENT, run);
  return () => {
    clearInterval(timer);
    document.removeEventListener("visibilitychange", run);
    window.removeEventListener("focus", run);
    window.removeEventListener(REFRESH_EVENT, run);
  };
}
