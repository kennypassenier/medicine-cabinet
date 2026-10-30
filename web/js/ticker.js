// Keeps time-dependent screens honest: runs `fn` every minute while the tab
// is visible and again the moment the tab or window regains focus, so a
// "due" dose never shows stale.

/** Refresh period in milliseconds. */
export const REFRESH_MS = 60_000;

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
  return () => {
    clearInterval(timer);
    document.removeEventListener("visibilitychange", run);
    window.removeEventListener("focus", run);
  };
}
