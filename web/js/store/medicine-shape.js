// How a medicine created on this device looks until the server has accepted it:
// the POST body completed with the server's own defaults and normalisation
// (docs/API.md "Medicine body"), so the pages can draw it like a served one.
// The derived fields (freshness, stock level, ...) are added by plan/judge.
// Pure; registered as the kit's overlay function of the medicines collection.

/** Trimmed, lower-cased, de-duplicated, order kept. @param {unknown} list */
export function cleanSymptoms(list) {
  const seen = new Set();
  for (const raw of Array.isArray(list) ? list : []) {
    const name = String(raw).trim().toLowerCase();
    if (name) seen.add(name);
  }
  return [...seen];
}

/** A slot with the server's defaults: every day, one dose. @param {any} s */
const slotOf = (s) => ({ time: s.time, days: s.days ?? 127, dose: s.dose ?? 1 });

/** A blank string counts as "none" for the dates. @param {unknown} v */
const dateOrNull = (v) => (typeof v === "string" && v.trim() !== "" ? v.trim() : null);

/** @param {any} b the POST body of a medicine create */
export function medicineFromBody(b) {
  return {
    id: b.id, name: String(b.name ?? "").trim(), purpose: b.purpose ?? "", form_id: b.form_id ?? null, form: "",
    strength: b.strength ?? "", notes: b.notes ?? "", active: b.active ?? true, as_needed: b.as_needed ?? false,
    min_gap_minutes: b.min_gap_minutes ?? null, max_per_day: b.max_per_day ?? null, stock: b.stock ?? 0,
    stock_unit: b.stock_unit ?? "", low_stock_at: b.low_stock_at ?? null, expires_on: dateOrNull(b.expires_on),
    opened_on: dateOrNull(b.opened_on), shelf_life_days: b.shelf_life_days ?? null, course_ends_on: dateOrNull(b.course_ends_on),
    symptoms: cleanSymptoms(b.symptoms),
    slots: (b.slots ?? []).map(slotOf).sort((x, y) => (x.time < y.time ? -1 : x.time > y.time ? 1 : 0)),
  };
}

/**
 * The kit's overlay function: a queued create is shaped from its body; a queued
 * edit keeps the kit's default (stored record with the body over it), with the
 * symptom names cleaned the way the server will.
 * @param {any | null} record @param {{ method: string, body?: any }} entry
 */
export function shapeMedicine(record, entry) {
  if (record === null) return medicineFromBody(entry.body ?? {});
  const next = { ...record, ...entry.body };
  return entry.body?.symptoms ? { ...next, symptoms: cleanSymptoms(entry.body.symptoms) } : next;
}
