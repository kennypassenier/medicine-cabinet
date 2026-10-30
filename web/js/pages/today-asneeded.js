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
 * @param {HTMLElement} into
 * @param {any[]} meds
 * @param {(m: any) => void} take
 */
export function renderAsNeeded(into, meds, take) {
  into.replaceChildren(h("ul", { class: "mc-list" }, ...meds.map((m) => {
    const why = blockedReason(m);
    const btn = button("I took one", { look: why ? "" : "kp-button--primary", key: `need-${m.id}`, onClick: () => take(m), label: `I took one ${m.name}` });
    if (why) {
      btn.setAttribute("aria-disabled", "true");
      btn.setAttribute("data-kp-disabled-reason", why);
      btn.disabled = true;
    }
    return h("li", { class: "mc-need" },
      h("div", { class: "mc-need__main" },
        h("a", { class: "mc-need__name", href: `/medicine/${m.id}` }, m.name),
        h("span", { class: "mc-need__meta" }, [m.purpose, `${qty(m.stock, m.stock_unit)} left`].filter(Boolean).join(" · "))),
      btn,
      h("span", { class: "mc-need__badges" }, ...statusBadges(m)));
  })));
}
