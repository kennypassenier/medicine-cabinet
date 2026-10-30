// Words and numbers for medicines: quantities with their unit, badges for
// stock / freshness / dose state, one place so every screen says it alike.

import { h } from "./dom.js";
import { formatDate, friendlyDate, daysFromToday } from "./clock.js";

/** 2 -> "2", 0.5 -> "0.5", 1.25 -> "1.25" (no trailing zeros, never exponent). @param {number} n */
export const num = (n) => String(Number(n.toFixed(2)));

/** "2 tablets" / "1 tablet" / "0.5 ml". @param {number} n @param {string} unit */
export function qty(n, unit) {
  const u = unit || "";
  const one = n === 1 && u.endsWith("s") ? u.slice(0, -1) : u;
  return `${num(n)} ${one}`.trim();
}

/** @param {string} cls @param {string} text @param {string} [title] */
export const badge = (cls, text, title) => h("span", { class: `kp-badge ${cls}`.trim(), title }, text);

/** The stock badge of a medicine, or null when stock is fine. @param {any} m */
export function stockBadge(m) {
  if (m.stock_level === "out") return badge("kp-badge--destructive", "Out of stock");
  if (m.stock_level === "low") return badge("kp-badge--warning", "Low stock");
  return null;
}

/** The freshness badge, or null when fresh. @param {any} m */
export function freshnessBadge(m) {
  const on = m.effective_expiry;
  if (m.freshness === "expired") return badge("kp-badge--destructive", "Expired", on ? `Expired on ${formatDate(on)}` : undefined);
  if (m.freshness === "expiring_soon") return badge("kp-badge--warning", on ? `Good until ${friendlyDate(on)}` : "Expiring soon");
  return null;
}

/** All the status badges a medicine has, worst first. @param {any} m */
export function statusBadges(m) {
  const list = [];
  if (!m.active) list.push(badge("", "Switched off"));
  if (m.course_over) list.push(badge("", "Course over"));
  list.push(stockBadge(m), freshnessBadge(m));
  return list.filter(Boolean);
}

/** "Strength, form" subtitle such as "400 mg tablet". @param {any} m */
export const specLine = (m) => [m.strength, m.form].filter(Boolean).join(" ");

/** Whole-day countdown sentence for a date, e.g. "ends in 3 days". @param {string} iso */
export function countdown(iso) {
  const n = daysFromToday(iso);
  if (n === 0) return "today";
  if (n === 1) return "tomorrow";
  if (n > 0) return `in ${n} days`;
  return `${-n} ${n === -1 ? "day" : "days"} ago`;
}

/** The server's sentences carry ISO dates; the screens show dd/mm/yyyy. @param {string} text */
export const humanize = (text) => text.replace(/(\d{4})-(\d{2})-(\d{2})/g, "$3/$2/$1");
