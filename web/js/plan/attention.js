// What needs a person's attention in the cabinet: a port of
// src/attention.rs. Stock, expiry and course alerts, in a fixed order per
// medicine.

import { daysUntil, iso, parseDate } from "./date.js";

/** @param {string | null | undefined} text */
function courseEnd(text) {
  if (text == null) return null;
  try {
    return parseDate(text);
  } catch {
    return null;
  }
}

/**
 * @param {any[]} medicines medicines with derived fields (see judge) @param {import("./date.js").CalDate} today
 * @returns {{ medicine_id: number, medicine_name: string, kind: string, detail: string }[]}
 */
export function attentionFor(medicines, today) {
  const out = [];
  for (const m of medicines.filter((x) => x.active)) {
    /** @param {string} kind @param {string} detail */
    const push = (kind, detail) => out.push({ medicine_id: m.id, medicine_name: m.name, kind, detail });
    if (m.stock_level === "out") push("out_of_stock", "None left.");
    else if (m.stock_level === "low") push("low_stock", `Only ${m.stock} ${m.stock_unit} left.`);
    if (m.effective_expiry != null && m.freshness === "expired") push("expired", `Expired on ${m.effective_expiry}.`);
    else if (m.effective_expiry != null && m.freshness === "expiring_soon") push("expiring_soon", `Good until ${m.effective_expiry}.`);
    const end = courseEnd(m.course_ends_on);
    if (end) {
      const left = daysUntil(today, end);
      if (left >= 0 && left <= 3) push("course_ends_soon", `The course ends on ${iso(end)}.`);
    }
  }
  return out;
}
