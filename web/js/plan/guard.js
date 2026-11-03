// The as-needed dosing guard: a port of check_dose in src/domain/schedule.rs
// and of the lookup in api/intakes.rs::dose_check.

import { parseDate, parseMinutes, toDays } from "./date.js";

const DAY_MINUTES = 24 * 60;

/** The server looks at this many most recent taken doses. */
const RECENT_LIMIT = 50;

/**
 * @typedef {{ allowed: boolean, reason: string | null, wait_minutes: number | null }} DoseVerdict
 */

/**
 * Apply the minimum gap and the 24-hour maximum.
 * @param {number[]} minutesAgo how long ago each past dose was, in minutes
 * @param {number | null | undefined} minGapMinutes
 * @param {number | null | undefined} maxPerDay
 * @returns {DoseVerdict}
 */
export function checkDose(minutesAgo, minGapMinutes, maxPerDay) {
  if (minGapMinutes != null && minutesAgo.length) {
    const last = Math.min(...minutesAgo);
    const wait = minGapMinutes - last;
    if (wait > 0) {
      return { allowed: false, reason: `The last dose was ${last} minutes ago; wait ${wait} more.`, wait_minutes: wait };
    }
  }
  if (maxPerDay != null) {
    const inDay = minutesAgo.filter((m) => m < DAY_MINUTES);
    if (inDay.length >= maxPerDay) {
      const wait = inDay.length ? DAY_MINUTES - Math.max(...inDay) : null;
      return { allowed: false, reason: `${inDay.length} doses in the last 24 hours is the maximum (${maxPerDay}).`, wait_minutes: wait };
    }
  }
  return { allowed: true, reason: null, wait_minutes: null };
}

/**
 * Minutes before `now` of the taken doses of one medicine (the 50 most
 * recent by time); rows with an unreadable time and doses in the future are
 * skipped, as on the server.
 * @param {{ medicine_id: number, status: string, taken_at: string }[]} intakes
 * @param {number} medicineId @param {string} now "YYYY-MM-DDTHH:MM"
 */
export function minutesAgo(intakes, medicineId, now) {
  const nowDays = toDays(parseDate(now));
  const nowMinutes = parseMinutes(now);
  const taken = intakes.filter((i) => i.medicine_id === medicineId && i.status === "taken");
  taken.sort((a, b) => (a.taken_at < b.taken_at ? 1 : a.taken_at > b.taken_at ? -1 : 0));
  const out = [];
  for (const i of taken.slice(0, RECENT_LIMIT)) {
    let ago;
    try {
      ago = (nowDays - toDays(parseDate(i.taken_at))) * DAY_MINUTES + nowMinutes - parseMinutes(i.taken_at);
    } catch {
      continue; // a row written before times were validated must not block the guard
    }
    if (ago >= 0) out.push(ago);
  }
  return out;
}

/**
 * May an as-needed medicine be taken right now? Same answer as
 * GET /api/mc/medicines/:id/dose-check.
 * @param {{ id: number, min_gap_minutes?: number | null, max_per_day?: number | null }} medicine
 * @param {{ medicine_id: number, status: string, taken_at: string }[]} intakes
 * @param {string} now "YYYY-MM-DDTHH:MM"
 * @returns {DoseVerdict}
 */
export function doseCheck(medicine, intakes, now) {
  return checkDose(minutesAgo(intakes, medicine.id, now), medicine.min_gap_minutes, medicine.max_per_day);
}
