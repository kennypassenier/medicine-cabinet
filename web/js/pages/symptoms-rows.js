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
    ? linkButton("Open", `/medicine/${m.id}`, "kp-button--ghost")
    : m.as_needed
      ? button("I took one", { look: "kp-button--primary", key: `symptom-${m.id}`, onClick: () => on.take(m), label: `I took one ${m.name}` })
      : h("span", { class: "mc-result__scheduled" }, "Scheduled");
  return h("li", { class: "mc-result" },
    h("div", { class: "mc-result__main" },
      h("a", { class: "mc-need__name", href: `/medicine/${m.id}` }, m.name),
      h("span", { class: "mc-need__meta" }, fact),
      h("span", { class: "mc-need__meta" }, on.usable ? scheduleLine(m) : whyNot(m))),
    action,
    h("span", { class: "mc-need__badges" }, ...(on.usable ? statusBadges(m) : statusBadges(m).filter((b) => b.textContent !== "Switched off" && b.textContent !== "Expired" && b.textContent !== "Out of stock"))));
}
