// Can this browser use the local database? A private window or a blocked-site-data
// setting makes the kit fall back to memory, and a missing kit client says no too;
// the app then reads and writes over plain fetch (see online.js). Kept free of
// globals so it can be tested.

/** How long the kit may take to say where it keeps the database before the question is dropped. */
export const PERSISTENT_TIMEOUT_MS = 2500;

/** True for the kit's "no network" failures: those are not a blocked database. @param {any} e */
export const isOfflineFailure = (e) => Boolean(e?.offline) || e?.status === 0;

/**
 * Why the local database cannot be used, or null when it can. `kit.db.persistent` is a promise once configured
 * (true = IndexedDB, false = the in-memory fallback). A "no network" rejection or silence cannot tell, so it is no problem.
 * @param {any} kit the configured kit client, or nothing
 * @param {number} [timeoutMs]
 * @returns {Promise<string | null>}
 */
export async function localDatabaseProblem(kit, timeoutMs = PERSISTENT_TIMEOUT_MS) {
  if (!kit?.db) return "the offline data client is not loaded";
  const silence = new Promise((done) => setTimeout(done, timeoutMs, undefined));
  try {
    const persistent = await Promise.race([Promise.resolve(kit.db.persistent), silence]);
    return persistent === false ? "the browser keeps the database in memory only" : null;
  } catch (e) {
    if (isOfflineFailure(e)) return null;
    return `the database failed to open: ${/** @type {any} */ (e)?.message ?? e}`;
  }
}
