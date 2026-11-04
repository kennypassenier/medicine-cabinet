// Can this browser use the local database? A private window, a blocked-site-data
// setting or a missing kit client all say no; the app then reads and writes
// over plain fetch (see online.js). Kept free of globals so it can be tested.

/** How long the IndexedDB probe may take before the database counts as blocked. */
export const PROBE_TIMEOUT_MS = 3000;

const PROBE_NAME = "mc-probe";

/**
 * Open (and drop) a scratch database. Resolves false when the browser refuses or stays silent.
 * @param {IDBFactory | undefined | null} idb @param {number} [timeoutMs]
 * @returns {Promise<boolean>}
 */
export function probeIndexedDb(idb, timeoutMs = PROBE_TIMEOUT_MS) {
  if (!idb) return Promise.resolve(false);
  return new Promise((resolve) => {
    const timer = setTimeout(() => resolve(false), timeoutMs);
    const done = (/** @type {boolean} */ ok) => {
      clearTimeout(timer);
      resolve(ok);
    };
    try {
      const req = idb.open(PROBE_NAME);
      req.onerror = () => done(false);
      req.onblocked = () => done(false);
      req.onsuccess = () => {
        req.result.close();
        try {
          idb.deleteDatabase(PROBE_NAME);
        } catch (e) {
          console.warn("The IndexedDB probe database could not be removed.", e);
        }
        done(true);
      };
    } catch {
      done(false);
    }
  });
}

/**
 * Why the local database cannot be used, or null when it can.
 * @param {{ kit: unknown, indexedDB: IDBFactory | undefined | null }} env
 * @param {(idb: IDBFactory | undefined | null) => Promise<boolean>} [probe]
 * @returns {Promise<string | null>}
 */
export async function localDatabaseProblem({ kit, indexedDB }, probe = probeIndexedDb) {
  if (!kit) return "the offline data client is not loaded";
  if (!indexedDB) return "indexedDB is not available";
  if (!(await probe(indexedDB))) return "indexedDB cannot be opened";
  return null;
}

/** True for the kit's "no network" failures: those are not a blocked database. @param {any} e */
export const isOfflineFailure = (e) => Boolean(e?.offline) || e?.status === 0;
