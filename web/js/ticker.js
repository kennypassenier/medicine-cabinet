// Keeps time-dependent screens honest: runs `fn` on the kit's refresh tick (one
// control, one remembered interval, shared with Status and Storage) and again
// the moment the tab or window regains focus, so a "due" dose never shows stale.

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
  const off = window.chassisRefresh?.onTick(async () => {
    await fn();
  });
  document.addEventListener("visibilitychange", run);
  window.addEventListener("focus", run);
  window.addEventListener(REFRESH_EVENT, run);
  return () => {
    off?.();
    document.removeEventListener("visibilitychange", run);
    window.removeEventListener("focus", run);
    window.removeEventListener(REFRESH_EVENT, run);
  };
}

/** Tell the kit's freshness line that the data on screen is current. */
export const markFresh = () => window.chassisRefresh?.markFresh(new Date());
