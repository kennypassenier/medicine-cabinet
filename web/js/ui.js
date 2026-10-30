// Shared view helpers built only from kp-themes parts: toasts, the page
// header and the card that holds loading / empty / failed / filled.

import { toast } from "/static/kp/js/overlays.js";
import { setCardState } from "/static/kp/js/card-state.js";
import { h } from "./dom.js";
import { ApiError } from "./api.js";

/** The server's sentence and its remedy, joined as one human line. @param {unknown} e */
export function errorText(e) {
  if (e instanceof ApiError) return e.remedy ? `${e.message} ${e.remedy}` : e.message;
  return e instanceof Error ? e.message : String(e);
}

/** Show a failure as an assertive toast; nothing is ever swallowed. @param {string} what @param {unknown} e */
export function reportError(what, e) {
  console.error(what, e);
  toast(`${what} ${errorText(e)}`, { live: "assertive", close: true, ms: 9000 });
}

/** A short polite confirmation. @param {string} text */
export const say = (text) => toast(text, { ms: 3500 });

/**
 * The page header: title and one sentence on the left, the actions on the right.
 * @param {{ title: string, description: string, actions?: HTMLElement[] }} p
 */
export function pageHeader({ title, description, actions = [] }) {
  return h("header", { class: "kp-page-header" },
    h("div", { class: "kp-page-header__inner" },
      h("div", null, h("div", { class: "kp-page-header__heading" }, h("div", { class: "kp-page-header__line" }, h("h1", { class: "kp-page-header__title", id: "page-title", tabindex: "-1" }, title))), h("p", { class: "kp-page-header__description" }, description)),
      actions.length ? h("div", { class: "kp-page-header__actions" }, ...actions) : null));
}

/**
 * A card with a heading and a sentence that owns its data states. `state()`
 * moves it between loading / error / empty / ready without moving anything.
 * @param {{ id: string, title: string, description?: string, actions?: HTMLElement[], className?: string, onRetry?: () => void }} o
 */
export function dataCard({ id, title, description, actions = [], className = "", onRetry }) {
  const data = h("div", { class: "kp-card__data" });
  const card = h("section", { class: `kp-card ${className}`.trim(), "data-kp-state": "loading", "aria-labelledby": `${id}-t` },
    h("div", { class: "kp-card__header" },
      h("h2", { class: "kp-card__title", id: `${id}-t` }, title),
      actions.length ? h("div", { class: "kp-card__actions" }, ...actions) : null),
    description ? h("p", { class: "kp-card__body" }, description) : null,
    h("div", { class: "kp-card__stage" }, data));
  card.addEventListener("click", (e) => {
    if (e.target instanceof Element && e.target.closest("[data-kp-card-retry]")) onRetry?.();
  });
  /** @param {"loading" | "error" | "empty" | "ready"} s @param {object} [opts] */
  const state = (s, opts) => setCardState(card, s, opts);
  return { card, data, state };
}

/** A button. @param {string} label @param {{ look?: string, onClick?: (e: Event) => void, key?: string, title?: string, label?: string }} [o] */
export function button(label, { look = "", onClick, key, title, label: aria } = {}) {
  return h("button", { type: "button", class: `kp-button ${look}`.trim(), "data-key": key, title, "aria-label": aria, onclick: onClick }, label);
}

/** A link styled as a button. @param {string} label @param {string} href @param {string} [look] */
export const linkButton = (label, href, look = "") => h("a", { class: `kp-button ${look}`.trim(), href }, label);
