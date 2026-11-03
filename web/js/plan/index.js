// The Today page computed in the browser from locally stored data. Pure:
// plain data and a local `now` ("YYYY-MM-DDTHH:MM") go in, the same JSON as
// GET /api/mc/today and GET /api/mc/medicines/:id/dose-check comes out.

import { parseDate, parseMinutes } from "./date.js";
import { attentionFor } from "./attention.js";
import { doseCheck } from "./guard.js";
import { judge } from "./medicine.js";
import { daySlots, plan } from "./plan.js";

export { PlanError, parseDate, parseMinutes, parseDatetime } from "./date.js";
export { validateSlot, runsOn } from "./schedule.js";
export { doseCheck, checkDose } from "./guard.js";
export { judge } from "./medicine.js";

/** ASCII-only lower-casing, like SQLite's NOCASE. @param {string} s */
const nocase = (s) => s.replace(/[A-Z]/g, (c) => c.toLowerCase());

/** The server's list order: switched-on first, then by name. @param {any} a @param {any} b */
function serverOrder(a, b) {
  if (a.active !== b.active) return a.active ? -1 : 1;
  const x = nocase(a.name);
  const y = nocase(b.name);
  return x < y ? -1 : x > y ? 1 : 0;
}

/**
 * What /api/mc/today returns.
 * @param {any[]} medicines stored medicine bodies (not deleted), with their slots
 * @param {any[]} intakes the logged intakes of today (any extra ones are harmless)
 * @param {string} now local "YYYY-MM-DDTHH:MM"
 */
export function overview(medicines, intakes, now) {
  const today = parseDate(now);
  const minute = parseMinutes(now);
  const judged = [...medicines].sort(serverOrder).map((m) => judge(m, today));
  return {
    doses: plan(daySlots(judged, intakes, today), minute),
    attention: attentionFor(judged, today),
    as_needed: judged.filter((m) => m.active && m.as_needed && !m.course_over),
  };
}
