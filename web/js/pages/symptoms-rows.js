// Result rows of the symptom search: what you have, and what you do not
// (with the reason, in words).

import { h } from "../dom.js";
import { formatDate } from "../clock.js";
import { qty, specLine, statusBadges } from "../format.js";
import { button, linkButton } from "../ui.js";
import { scheduleLine } from "./cabinet-row.js";

/** Why a medicine is not usable, in one plain sentence. @param {any} m */
export function whyNot(m) {
  if (!m.active) return "Switched off in the cabinet.";
  if (m.stock_level === "out") return "Out of stock.";
  if (m.freshness === "expired") return `Expired${m.effective_expiry ? ` on ${formatDate(m.effective_expiry)}` : ""}.`;
  if (m.course_over) return "Its course is over.";
  return "Not available right now.";
}

/** @param {any} m @param {{ take: (m: any) => void, usable: boolean }} on */
export function resultRow(m, on) {
  const fact = [specLine(m), m.purpose, `${qty(m.stock, m.stock_unit)} left`].filter(Boolean).join(" · ");
  const action = !on.usable
    ? linkButton("Open", `/medicine/${m.id}`, "kp-button--ghost kp-w-full")
    : m.as_needed
      ? button("I took one", { look: "kp-button--primary kp-w-full", key: `symptom-${m.id}`, onClick: () => on.take(m), label: `I took one ${m.name}` })
      : h("span", { class: "kp-text-muted" }, "Scheduled");
  const badges = on.usable ? statusBadges(m) : statusBadges(m).filter((b) => !["Switched off", "Expired", "Out of stock"].includes(b.textContent ?? ""));
  return h("li", { class: "kp-listrow" },
    h("div", { class: "kp-listrow__identity" },
      h("a", { class: "kp-listrow__name", href: `/medicine/${m.id}` }, m.name),
      h("span", { class: "kp-listrow__meta" }, fact),
      h("span", { class: "kp-listrow__meta" }, on.usable ? scheduleLine(m) : whyNot(m))),
    h("div", { class: "kp-listrow__status" }, h("div", { class: "kp-badges" }, ...badges)),
    h("div", { class: "kp-listrow__stepper" }, action));
}
