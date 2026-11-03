// Calendar arithmetic on plain "YYYY-MM-DD" strings: a port of
// src/domain/date.rs. Pure: no Date object, no clock, no time zone. The
// caller passes its local "now" as "YYYY-MM-DDTHH:MM".

/** The years the app accepts (same bounds as the server). */
export const MIN_YEAR = 1900;
export const MAX_YEAR = 2200;

/** A strict parse failure; the message says what to type instead. */
export class PlanError extends Error {
  /** @param {string} message @param {string} hint */
  constructor(message, hint) {
    super(message);
    this.name = "PlanError";
    this.hint = hint;
  }
}

/** @typedef {{ year: number, month: number, day: number }} CalDate */

/** Cut user text down for an error message. @param {string} text */
const shown = (text) => String(text).slice(0, 40);

/** @param {string} text */
function badDate(text) {
  return new PlanError(`'${shown(text)}' is not a date`, "use the form YYYY-MM-DD, for example 2026-10-30, with a year from 1900 to 2200");
}

/** @param {number} year @param {number} month */
function daysInMonth(year, month) {
  if (month === 2) return year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0) ? 29 : 28;
  return [4, 6, 9, 11].includes(month) ? 30 : 31;
}

/**
 * Parse a strict YYYY-MM-DD; a time part may follow after a "T" or a space
 * and is ignored here. @param {string} text @returns {CalDate}
 */
export function parseDate(text) {
  if (typeof text !== "string" || !/^[0-9]{4}-[0-9]{2}-[0-9]{2}(?:[T ]|$)/.test(text)) throw badDate(String(text));
  const year = Number(text.slice(0, 4));
  const month = Number(text.slice(5, 7));
  const day = Number(text.slice(8, 10));
  const inRange = year >= MIN_YEAR && year <= MAX_YEAR && month >= 1 && month <= 12;
  if (!inRange || day === 0 || day > daysInMonth(year, month)) throw badDate(text);
  return { year, month, day };
}

/** Days since 1970-01-01 (Howard Hinnant's civil-days algorithm). @param {CalDate} d */
export function toDays(d) {
  const y = d.year - (d.month <= 2 ? 1 : 0);
  const era = Math.floor(y / 400);
  const yoe = y - era * 400;
  const doy = Math.floor((153 * (d.month > 2 ? d.month - 3 : d.month + 9) + 2) / 5) + d.day - 1;
  const doe = yoe * 365 + Math.floor(yoe / 4) - Math.floor(yoe / 100) + doy;
  return era * 146097 + doe - 719468;
}

/** @param {number} days @returns {CalDate} */
export function fromDays(days) {
  const z = days + 719468;
  const era = Math.floor(z / 146097);
  const doe = z - era * 146097;
  const yoe = Math.floor((doe - Math.floor(doe / 1460) + Math.floor(doe / 36524) - Math.floor(doe / 146096)) / 365);
  const doy = doe - (365 * yoe + Math.floor(yoe / 4) - Math.floor(yoe / 100));
  const mp = Math.floor((5 * doy + 2) / 153);
  const day = doy - Math.floor((153 * mp + 2) / 5) + 1;
  const month = mp < 10 ? mp + 3 : mp - 9;
  return { year: yoe + era * 400 + (month <= 2 ? 1 : 0), month, day };
}

/** Whole days from `a` to `b` (negative when `b` is earlier). @param {CalDate} a @param {CalDate} b */
export const daysUntil = (a, b) => toDays(b) - toDays(a);

/** @param {CalDate} d @param {number} n */
export const plusDays = (d, n) => fromDays(toDays(d) + n);

/** 0 = Monday ... 6 = Sunday. @param {CalDate} d */
export const weekday = (d) => (((toDays(d) + 3) % 7) + 7) % 7;

/** "YYYY-MM-DD". @param {CalDate} d */
export const iso = (d) => `${String(d.year).padStart(4, "0")}-${String(d.month).padStart(2, "0")}-${String(d.day).padStart(2, "0")}`;

/** Earlier of two dates. @param {CalDate} a @param {CalDate} b */
export const minDate = (a, b) => (toDays(a) <= toDays(b) ? a : b);

/** @param {string} text */
function badTime(text) {
  return new PlanError(`'${shown(text)}' is not a time`, "use the form HH:MM, for example 08:30");
}

/**
 * Minutes since midnight from HH:MM, or from the time part of
 * YYYY-MM-DDTHH:MM (seconds after it are ignored).
 * @param {string} text
 */
export function parseMinutes(text) {
  if (typeof text !== "string") throw badTime(String(text));
  let t;
  if (text.length === 5) t = text;
  else if (text.length > 10) t = text.slice(11);
  else throw badTime(text);
  if (!/^[0-9]{2}:[0-9]{2}(?::|$)/.test(t)) throw badTime(text);
  const h = Number(t.slice(0, 2));
  const m = Number(t.slice(3, 5));
  if (h > 23 || m > 59) throw badTime(text);
  return h * 60 + m;
}

/** "HH:MM" for a minute of the day (the canonical form stored for slots). @param {number} minutes */
export const clockText = (minutes) => `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`;

/** Canonical "YYYY-MM-DDTHH:MM" from what the browser sent, or a PlanError. @param {string} text */
export function parseDatetime(text) {
  const date = parseDate(text);
  if (text.length < 16) throw new PlanError(`'${shown(text)}' has no time`, "use the form YYYY-MM-DDTHH:MM, for example 2026-10-30T08:30");
  return `${iso(date)}T${clockText(parseMinutes(text))}`;
}
