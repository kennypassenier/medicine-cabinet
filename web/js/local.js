// Starts the kit's local database: the app works from what is stored, so it
// never waits on the network longer than a moment. Offline, the screens draw
// from the last sync and the kit shows its connection band and failed-edits list.

import { toast } from "/static/kp/dist/kp-themes.min.js";
import { isOfflineFailure, localDatabaseProblem } from "./store/availability.js";
import { useOnlineStore } from "./store/index.js";
import { shapeMedicine } from "./store/medicine-shape.js";
import { refreshNow } from "./ticker.js";

const FIRST_SYNC_MS = 2500;
/** How long the online-only notice stays up. */
const ONLINE_NOTICE_MS = 15000;
const ONLINE_NOTICE = "Working online only: this browser blocks the offline database.";

/** Names a queued write in the kit's failed-edits list. @param {{ method: string, path: string, body?: any }} entry */
function describe({ method, path, body }) {
  const p = path.split("?")[0].replace("/api/mc/", "");
  const named = body?.medicine_name ?? body?.name;
  if (p === "intakes") return { title: `${body?.status === "skipped" ? "Skip" : "Take"} ${named ?? "a dose"}` };
  if (p.startsWith("intakes/")) return { title: method === "DELETE" ? "Remove a logged dose" : "Change a logged dose" };
  if (p.endsWith("/stock")) return { title: "Add stock", detail: `${body?.delta > 0 ? "+" : ""}${body?.delta}` };
  if (p.endsWith("/active")) return { title: body?.active ? "Switch a medicine on" : "Switch a medicine off" };
  if (p.endsWith("/restore")) return { title: "Restore a medicine" };
  const [kind] = p.split("/");
  const verb = method === "POST" ? "Add" : method === "DELETE" ? "Delete" : "Change";
  const noun = { medicines: "medicine", symptoms: "symptom", categories: "category", forms: "form" }[kind] ?? "item";
  return { title: `${verb} ${noun}${named ? ` ${named}` : ""}` };
}

/**
 * The overlay drops a refused write from the views by itself (the kit leaves
 * failed entries out of it), but a screen already drawn keeps the row it drew
 * from the queued create. When the failed count grows, every screen redraws.
 */
function redrawOnRefusal() {
  let failed = 0;
  window.addEventListener("chassis-data-queue", (e) => {
    const now = /** @type {CustomEvent} */ (e).detail?.failed ?? 0;
    if (now > failed) refreshNow();
    failed = now;
  });
}

/** Serve the pages over plain fetch and say so. @param {string} why */
function goOnline(why) {
  console.warn(`The local database is not usable (${why}); working online only.`);
  useOnlineStore();
  toast(ONLINE_NOTICE, { close: true, ms: ONLINE_NOTICE_MS });
}

/** Start the kit's database; resolves with the reason it cannot be used, or null. @param {any} kit */
async function openDatabase(kit) {
  await kit.configure({ name: "medicine-cabinet", db: { overlay: { medicines: shapeMedicine } }, describe });
  // The cabinet is a few kilobytes: keep all of it on the device.
  if (!(await kit.settings()).all) await kit.keepAll(true);
  const problem = await localDatabaseProblem(kit);
  if (problem) return problem;
  const opened = kit.db.ready().then(() => null, (/** @type {any} */ e) => e);
  const deadline = new Promise((done) => setTimeout(done, FIRST_SYNC_MS));
  const failure = await Promise.race([opened, deadline.then(() => null)]);
  if (failure && !isOfflineFailure(failure)) return `the database failed to open: ${failure?.message ?? failure}`;
  const first = opened.then((e) => (e ? undefined : kit.db.sync()));
  first.catch((e) => console.warn("The first sync did not finish; showing the stored copy.", e));
  await Promise.race([first.catch(() => {}), deadline]);
  return null;
}

/** Configure the kit and wait (briefly) for the first sync. Never rejects: offline the app draws from the stored copy; with the database blocked it works online only. */
export async function startLocal() {
  const kit = /** @type {any} */ (window).chassisData;
  if (!kit) return goOnline("the offline data client is not loaded");
  redrawOnRefusal();
  try {
    const failed = await openDatabase(kit);
    if (failed) goOnline(failed);
  } catch (e) {
    console.error("The local database could not be started.", e);
    goOnline(String(/** @type {any} */ (e)?.message ?? e));
  }
}
