// The judgements derived from a medicine and today's date: a port of
// effective_expiry, freshness and stock_level in src/domain/medicine.rs and
// of the course_over rule in store/medicines.rs. Stored bodies carry these
// fields as of the day they were served, so they are recomputed locally.

import { daysUntil, iso, minDate, parseDate, plusDays } from "./date.js";

/** Days before an expiry that count as "expiring soon". */
const SOON_DAYS = 30;

/** @param {string | null | undefined} text */
function tryDate(text) {
  if (text == null) return null;
  try {
    return parseDate(text);
  } catch {
    return null;
  }
}

/**
 * The earlier of the printed expiry and "opened + shelf life".
 * @param {string | null | undefined} expiresOn @param {string | null | undefined} openedOn
 * @param {number | null | undefined} shelfLifeDays
 */
export function effectiveExpiry(expiresOn, openedOn, shelfLifeDays) {
  const printed = tryDate(expiresOn);
  const opened = shelfLifeDays != null ? tryDate(openedOn) : null;
  const afterOpening = opened && shelfLifeDays != null ? plusDays(opened, shelfLifeDays) : null;
  if (printed && afterOpening) return minDate(printed, afterOpening);
  return printed ?? afterOpening;
}

/** "fresh" | "expiring_soon" | "expired". @param {import("./date.js").CalDate | null} expiry @param {import("./date.js").CalDate} today */
export function freshness(expiry, today) {
  if (!expiry) return "fresh";
  const left = daysUntil(today, expiry);
  if (left < 0) return "expired";
  return left <= SOON_DAYS ? "expiring_soon" : "fresh";
}

/** "out" at zero, "low" at or under the threshold, else "ok". @param {number} stock @param {number | null | undefined} lowAt */
export function stockLevel(stock, lowAt) {
  if (stock <= 0) return "out";
  return lowAt != null && stock <= lowAt ? "low" : "ok";
}

/**
 * The medicine with its derived fields recomputed for `today`.
 * @param {any} medicine a stored medicine body @param {import("./date.js").CalDate} today
 */
export function judge(medicine, today) {
  const expiry = effectiveExpiry(medicine.expires_on, medicine.opened_on, medicine.shelf_life_days);
  const end = tryDate(medicine.course_ends_on);
  return {
    ...medicine,
    effective_expiry: expiry ? iso(expiry) : null,
    freshness: freshness(expiry, today),
    stock_level: stockLevel(medicine.stock, medicine.low_stock_at),
    course_over: end !== null && daysUntil(end, today) > 0,
  };
}
