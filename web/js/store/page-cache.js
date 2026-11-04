// Online-only mode: one call per page load. A page asks for several resources
// at once (Today: today + medicines); the first ask fetches the merged
// /api/mc/pages/<name> answer and the others take their part of it. A part is
// handed out once and the answer expires quickly, so a refresh fetches again.
// Pure: the fetch, the clock and the current page are injected.

/** How long a fetched page answer may serve the other asks of the same load. */
export const BUNDLE_TTL_MS = 2000;

/** The parts each merged endpoint answers (docs/API.md "Merged page endpoints"). */
export const PAGE_PARTS = {
  today: ["today", "medicines"],
  cabinet: ["medicines", "forms", "categories"],
  symptoms: ["symptoms", "categories", "medicines"],
  history: ["intakes", "medicines"],
  settings: ["forms", "categories", "symptoms"],
};

const PATHS = /** @type {Record<string, keyof typeof PAGE_PARTS>} */ ({
  "/": "today",
  "/cabinet": "cabinet",
  "/symptoms": "symptoms",
  "/history": "history",
  "/settings": "settings",
});

/** The page a path shows, if it has a merged endpoint. @param {string} pathname */
export function pageAt(pathname) {
  return PATHS[pathname.replace(/\/+$/, "") || "/"];
}

/**
 * @param {{ fetchPage: (page: string) => Promise<any>, currentPage: () => string | undefined, now?: () => number, ttlMs?: number }} deps
 */
export function createPageCache({ fetchPage, currentPage, now = Date.now, ttlMs = BUNDLE_TTL_MS }) {
  /** @type {Map<string, { at: number, taken: Set<string>, answer: Promise<any> }>} */
  const entries = new Map();

  /** @param {string} page */
  function start(page) {
    const entry = { at: now(), taken: new Set(), answer: fetchPage(page) };
    entries.set(page, entry);
    // A failed fetch is not kept: the next ask tries again.
    entry.answer.catch(() => entries.get(page) === entry && entries.delete(page));
    return entry;
  }

  return {
    /**
     * One resource for the page on show: from the merged answer when the page has one for it,
     * otherwise from `single`. @template T @param {string} part @param {() => Promise<T>} single @param {boolean} [fits] false: this ask is not the page's own (another day, another limit)
     * @returns {Promise<T>}
     */
    part(part, single, fits = true) {
      const page = currentPage();
      if (!page || !fits || !PAGE_PARTS[page].includes(part)) return single();
      let entry = entries.get(page);
      if (!entry || entry.taken.has(part) || now() - entry.at > ttlMs) entry = start(page);
      entry.taken.add(part);
      return entry.answer.then((a) => a[part]);
    },
    /** Forget every answer: a write happened. */
    clear: () => entries.clear(),
  };
}
