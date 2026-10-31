// The "when needed" card: one tap logs a dose after the server's guard agrees.

import { h } from "../dom.js";
import { qty, statusBadges } from "../format.js";
import { button } from "../ui.js";

/** Why an as-needed medicine cannot be taken, or null. @param {any} m */
export function blockedReason(m) {
  if (m.stock_level === "out") return "Out of stock";
  if (m.freshness === "expired") return "Expired";
  if (m.course_over) return "Course is over";
  return null;
}

/**
 * One list row per medicine: identity, status badges and the button in the stepper column.
 * @param {HTMLElement} into
 * @param {any[]} meds
 * @param {(m: any) => void} take
 */
export function renderAsNeeded(into, meds, take) {
  into.replaceChildren(h("ul", { class: "kp-list", "aria-label": "Medicines taken when needed" }, ...meds.map((m) => {
    const why = blockedReason(m);
    const btn = button("I took one", { look: `kp-w-full ${why ? "" : "kp-button--primary"}`.trim(), key: `need-${m.id}`, onClick: () => take(m), label: `I took one ${m.name}` });
    if (why) {
      btn.setAttribute("aria-disabled", "true");
      btn.setAttribute("data-kp-disabled-reason", why);
      btn.disabled = true;
    }
    return h("li", { class: "kp-listrow" },
      h("div", { class: "kp-listrow__identity" },
        h("a", { class: "kp-listrow__name", href: `/medicine/${m.id}` }, m.name),
        h("span", { class: "kp-listrow__meta" }, [m.purpose, `${qty(m.stock, m.stock_unit)} left`].filter(Boolean).join(" · "))),
      h("div", { class: "kp-listrow__status" }, h("div", { class: "kp-badges" }, ...statusBadges(m))),
      h("div", { class: "kp-listrow__stepper" }, btn));
  })));
}
