// Repeating schedule slots: a port of SlotInput in src/domain/schedule.rs.

import { PlanError, clockText, parseMinutes, weekday } from "./date.js";

/** @typedef {{ time: string, days?: number, dose?: number }} Slot */

/** Largest dose or amount, as the server's limits. */
const AMOUNT_MAX = 1_000_000;

/** Bit 0 = Monday ... bit 6 = Sunday; 127 = every day. */
export const EVERY_DAY = 0b111_1111;

/** Does the slot run on this date? @param {Slot} slot @param {import("./date.js").CalDate} date */
export const runsOn = (slot, date) => ((slot.days ?? EVERY_DAY) & (1 << weekday(date))) !== 0;

/**
 * Check a slot like the server does and return it with a canonical time.
 * A missing `days` means every day, a missing `dose` one.
 * @param {Slot} slot @returns {Required<Slot>}
 */
export function validateSlot(slot) {
  const time = clockText(parseMinutes(String(slot.time).trim()));
  const days = slot.days ?? EVERY_DAY;
  if (!Number.isInteger(days) || days <= 0 || days > EVERY_DAY) {
    throw new PlanError("a schedule slot needs at least one weekday", "pick one or more days");
  }
  const dose = slot.dose ?? 1;
  if (!Number.isFinite(dose) || dose <= 0 || dose > AMOUNT_MAX) {
    throw new PlanError(`a dose must be a number more than 0 and at most ${AMOUNT_MAX}`, `enter a number from more than 0 up to ${AMOUNT_MAX}`);
  }
  return { time, days, dose };
}
