// Turning schedule slots and logged intakes into a day plan: a port of
// src/domain/plan.rs and src/dayplan.rs.

import { iso, parseMinutes } from "./date.js";
import { runsOn } from "./schedule.js";

/** A dose is "due" from its time until this many minutes later; after that it is overdue. */
export const DUE_WINDOW_MINUTES = 60;

/**
 * One scheduled dose of one medicine on one day.
 * @typedef {{
 *   medicine_id: number, medicine_name: string, minute: number, time: string, dose: number, unit: string,
 *   answered: { id: any, status: string } | null
 * }} PlanSlot
 */

/**
 * "upcoming" | "due" | "overdue" | "taken" | "skipped". An answer beats the clock.
 * @param {PlanSlot} slot @param {number} nowMinute
 */
export function stateOf(slot, nowMinute) {
  if (slot.answered) return slot.answered.status === "taken" ? "taken" : "skipped";
  const late = nowMinute - slot.minute;
  if (late < 0) return "upcoming";
  return late <= DUE_WINDOW_MINUTES ? "due" : "overdue";
}

/** @param {string} a @param {string} b code-point order, like Rust's String ordering */
const compareText = (a, b) => {
  const x = Array.from(a);
  const y = Array.from(b);
  for (let i = 0; i < Math.min(x.length, y.length); i++) {
    if (x[i] !== y[i]) return x[i].codePointAt(0) - y[i].codePointAt(0);
  }
  return x.length - y.length;
};

/**
 * The day's doses in time order (ties by medicine name).
 * @param {PlanSlot[]} slots @param {number} nowMinute
 */
export function plan(slots, nowMinute) {
  const sorted = [...slots].sort((a, b) => a.minute - b.minute || compareText(a.medicine_name, b.medicine_name));
  return sorted.map((s) => ({
    medicine_id: s.medicine_id,
    medicine_name: s.medicine_name,
    time: s.time,
    dose: s.dose,
    unit: s.unit,
    state: stateOf(s, nowMinute),
    intake_id: s.answered ? s.answered.id : null,
    minutes_until: s.minute - nowMinute,
  }));
}

/**
 * Scheduled slots that run today, paired with the intake that answered them
 * (the first intake of the same medicine planned for exactly that slot).
 * @param {any[]} medicines medicines with derived fields (see judge) @param {any[]} intakes @param {import("./date.js").CalDate} today
 * @returns {PlanSlot[]}
 */
export function daySlots(medicines, intakes, today) {
  const out = [];
  for (const m of medicines.filter((x) => x.active && !x.as_needed && !x.course_over)) {
    for (const s of (m.slots ?? []).filter((x) => runsOn(x, today))) {
      const planned = `${iso(today)}T${s.time}`;
      const hit = intakes.find((i) => i.medicine_id === m.id && i.planned_for === planned);
      out.push({
        medicine_id: m.id,
        medicine_name: m.name,
        minute: safeMinutes(s.time),
        time: s.time,
        dose: s.dose ?? 1,
        unit: m.stock_unit,
        answered: hit ? { id: hit.id, status: hit.status } : null,
      });
    }
  }
  return out;
}

/** Minutes of a stored slot time; 0 for an unreadable one, as on the server. @param {string} time */
function safeMinutes(time) {
  try {
    return parseMinutes(time);
  } catch {
    return 0;
  }
}

