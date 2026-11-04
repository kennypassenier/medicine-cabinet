// The intake log as the pages see it: newest first, each with its medicine's
// name. An intake whose medicine was purged has a null `medicine_id` and keeps
// its own stored `medicine_name`; it is never dropped. Pure.

/**
 * @param {any[]} rows named intakes @param {any[]} medicines stored medicines
 * @param {string | undefined} day prefix @param {number} limit
 */
export function listIntakes(rows, medicines, day, limit) {
  const names = new Map(medicines.map((m) => [Number(m.id), m.name]));
  const own = (/** @type {any} */ i) => (i.medicine_id == null ? undefined : names.get(Number(i.medicine_id)));
  return rows
    .filter((i) => !day || i.taken_at.startsWith(day))
    .map((i) => ({ ...i, medicine_name: own(i) ?? i.medicine_name ?? "" }))
    .sort((a, b) => (a.taken_at < b.taken_at ? 1 : a.taken_at > b.taken_at ? -1 : 0))
    .slice(0, Math.min(1000, Math.max(1, limit)));
}
