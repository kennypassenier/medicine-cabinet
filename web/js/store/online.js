// The data layer without the local database: the same function names and
// result shapes as ./index.js, served by plain fetch. A page load takes the
// merged /api/mc/pages/<name> answer (one call); other reads use the single
// endpoints; writes call the single endpoints directly, with no queue and no
// optimistic overlay. Used only when the browser blocks the local database.

import { nowParam } from "../clock.js";
import { call } from "./online-http.js";
import { createPageCache, pageAt } from "./page-cache.js";
import { created, intakeBody, intakeRows, loggedIntake, symptomBody } from "./online-shape.js";

/** The `limit` the merged history answer is read with (the server's default for `GET /intakes`). */
const MERGED_INTAKE_LIMIT = 200;

const cache = createPageCache({
  fetchPage: (page) => call("GET", `/pages/${page}`, { timed: true }),
  currentPage: () => pageAt(location.pathname),
});

/** @template T @param {string} part @param {string} path @param {boolean} [timed] */
const read = (part, path, timed = false) => cache.part(part, () => call("GET", path, { timed }));

/** A write: whatever the cache holds is stale afterwards, even when the write failed half-way. @template T @param {() => Promise<T>} fn @returns {Promise<T>} */
async function write(fn) {
  try {
    return await fn();
  } finally {
    cache.clear();
  }
}

const item = (/** @type {string} */ id) => `/medicines/${id}`;

export const onlineStore = {
  medicines: () => read("medicines", "/medicines", true),
  medicine: (/** @type {string} */ id) => call("GET", item(id), { timed: true }),
  today: () => read("today", "/today", true),
  search: (/** @type {string} */ symptom) => call("GET", "/search", { timed: true, query: { symptom } }),
  doseCheck: (/** @type {string} */ id) => call("GET", `${item(id)}/dose-check`, { timed: true }),
  create: (/** @type {object} */ body) => write(async () => created(await call("POST", "/medicines", { timed: true, body }))),
  update: (/** @type {string} */ id, /** @type {object} */ body) => write(() => call("PUT", item(id), { timed: true, body })),
  remove: (/** @type {string} */ id) => write(() => call("DELETE", item(id))),
  restore: (/** @type {string} */ id) => write(() => call("POST", `${item(id)}/restore`, { timed: true })),
  setActive: (/** @type {string} */ id, /** @type {boolean} */ active) => write(() => call("POST", `${item(id)}/active`, { timed: true, body: { active } })),
  addStock: (/** @type {string} */ id, /** @type {number} */ delta) => write(() => call("POST", `${item(id)}/stock`, { timed: true, body: { delta } })),

  intakes: async (/** @type {string | undefined} */ day, limit = MERGED_INTAKE_LIMIT) => {
    const own = day === undefined && limit === MERGED_INTAKE_LIMIT;
    const rows = await cache.part("intakes", () => call("GET", "/intakes", { query: { day, limit } }), own);
    return intakeRows(rows);
  },
  logIntake: (/** @type {Parameters<typeof intakeBody>[0] & { takenAt?: string }} */ i) =>
    write(async () => loggedIntake(await call("POST", "/intakes", { body: intakeBody({ ...i, takenAt: i.takenAt ?? nowParam() }) }))),
  undoIntake: (/** @type {string} */ id) => write(() => call("DELETE", `/intakes/${id}`)),
  editIntake: (/** @type {string} */ id, /** @type {{ taken_at: string, dose: number, note: string }} */ b) =>
    write(async () => created(await call("PUT", `/intakes/${id}`, { body: b }))),

  symptoms: () => read("symptoms", "/symptoms"),
  addSymptom: (/** @type {object} */ b) => write(async () => created(await call("POST", "/symptoms", { body: symptomBody(b) }))),
  editSymptom: (/** @type {string} */ id, /** @type {object} */ b) => write(async () => created(await call("PUT", `/symptoms/${id}`, { body: b }))),
  removeSymptom: (/** @type {string} */ id) => write(() => call("DELETE", `/symptoms/${id}`)),
  categories: () => read("categories", "/categories"),
  addCategory: (/** @type {string} */ name, /** @type {string[]} */ symptomIds = []) => write(async () => created(await call("POST", "/categories", { body: { name, symptom_ids: symptomIds } }))),
  editCategory: (/** @type {string} */ id, /** @type {string} */ name) => write(async () => created(await call("PUT", `/categories/${id}`, { body: { name } }))),
  removeCategory: (/** @type {string} */ id) => write(() => call("DELETE", `/categories/${id}`)),
  forms: () => read("forms", "/forms"),
  addForm: (/** @type {object} */ b) => write(async () => created(await call("POST", "/forms", { body: b }))),
  editForm: (/** @type {string} */ id, /** @type {object} */ b) => write(async () => created(await call("PUT", `/forms/${id}`, { body: b }))),
  removeForm: (/** @type {string} */ id) => write(() => call("DELETE", `/forms/${id}`)),
};
