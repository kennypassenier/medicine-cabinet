// The app shell: top bar, phone tab bar, theme picker and the history-API
// router. Each screen is a module with `mount(root, ctx)` returning its
// cleanup. The server answers index.html for every extension-less path, so
// a deep link (/medicine/3) lands here too.

import { attachNavToggles, attachNavMenus, attachSkipLinks } from "/static/kp/js/components.js";
import { attachThemePickers, themeMenuMarkup } from "/static/kp/js/theme-picker.js";
import { attachTabbars, setTabbarCurrent } from "/static/kp/js/tabbar.js";
import { attachSegmented } from "/static/kp/js/segmented.js";
import { applyStoredCalm } from "/static/kp/js/calm.js";
import { attachEffects } from "/static/kp/js/effects.js";
import { h } from "./dom.js";
import { AREAS, route, tabMarkup } from "./routes.js";
import { reportError } from "./ui.js";

const page = /** @type {HTMLElement} */ (document.getElementById("page"));
const nav = /** @type {HTMLElement} */ (document.getElementById("nav"));
const tabbar = /** @type {HTMLElement} */ (document.getElementById("tabbar"));
const tools = /** @type {HTMLElement} */ (document.getElementById("tools"));

applyStoredCalm();
// The themes' pointer effects (the dark theme's cursor glow) follow the mouse only once attached.
attachEffects(document);

/** @type {() => void} */
let cleanup = () => {};
let seq = 0;

/** @param {string} href */
export function navigate(href) {
  history.pushState(null, "", href);
  void render();
}

const trimPath = (/** @type {string} */ p) => p.replace(/\/+$/, "") || "/";

async function render() {
  const mine = ++seq;
  cleanup();
  cleanup = () => {};
  let found = route(trimPath(location.pathname), location.search);
  if (found && "redirect" in found) {
    history.replaceState(null, "", found.redirect);
    found = route(trimPath(location.pathname), location.search);
  }
  if (found && "redirect" in found) return;
  markNav(found?.bar ?? "");
  if (!found) {
    document.title = "Not found · Medicine cabinet";
    return void page.replaceChildren(plate("Page not found", "There is nothing at this address.", "Back to today", "/"));
  }
  let mod;
  try {
    mod = await found.load();
  } catch (e) {
    if (mine !== seq) return;
    reportError("This screen could not be loaded.", e);
    return void page.replaceChildren(plate("This screen could not be loaded", "Reload the page to try again.", "Reload", location.href));
  }
  if (mine !== seq) return;
  page.replaceChildren();
  const done = mod.mount(page, { params: found.params, navigate });
  if (typeof done === "function") cleanup = done;
  if (!location.hash) window.scrollTo({ top: 0 });
}

/** The plate for a screen that cannot be drawn at all. */
function plate(title, text, label, href) {
  return h("section", { class: "kp-card mc-plate" },
    h("div", { class: "kp-empty" }, h("h1", { class: "kp-empty__title" }, title), h("p", { class: "kp-empty__body" }, text), h("a", { class: "kp-button kp-button--primary", href: label === "Reload" ? location.pathname : href }, label)));
}

/** @param {string} path */
function markNav(path) {
  for (const a of nav.querySelectorAll("a")) {
    if (a.getAttribute("href") === path) a.setAttribute("aria-current", "page");
    else a.removeAttribute("aria-current");
  }
  setTabbarCurrent(tabbar, path);
}

/** The bar lists the app's areas, then the kit's own pages from the registry. */
async function drawNav() {
  const own = AREAS.map((a) => h("li", null, h("a", { class: "kp-nav__link", href: a.path }, a.title)));
  nav.replaceChildren(...own);
  try {
    const res = await fetch("/api/kit/pages", { credentials: "same-origin" });
    if (!res.ok) throw new Error(String(res.status));
    const set = await res.json();
    const kit = set.pages.filter((/** @type {any} */ p) => p.nav && p.source === "kit");
    nav.append(...kit.map((/** @type {any} */ p) => h("li", null, h("a", { class: "kp-nav__link", href: p.path, title: p.hint }, p.title))));
  } catch (e) {
    console.warn("kit page registry unavailable", e);
  }
  markNav(route(trimPath(location.pathname), location.search)?.bar ?? "");
}

const theme = h("span", { class: "mc-theme" });
theme.innerHTML = themeMenuMarkup({ id: "mc-theme-menu", label: "Theme" });
tools.replaceChildren(theme);
attachThemePickers(tools);
tabbar.replaceChildren(...AREAS.map(tabMarkup));
attachTabbars(document);
attachNavToggles(document);
attachNavMenus(document);
attachSkipLinks(document);
// Chips and pills are drawn by the screens after load; one listener on the document wires every group.
attachSegmented(document);

// Same-origin links to a screen of this app route in place; modified clicks open normally.
document.addEventListener("click", (e) => {
  const a = /** @type {HTMLElement} */ (e.target)?.closest?.("a[href]");
  if (!(a instanceof HTMLAnchorElement) || e.defaultPrevented || e.button !== 0) return;
  if (e.metaKey || e.ctrlKey || e.shiftKey || e.altKey || a.target || a.hasAttribute("download")) return;
  const url = new URL(a.href);
  if (url.origin !== location.origin || !route(trimPath(url.pathname), url.search)) return;
  e.preventDefault();
  navigate(url.pathname + url.search + url.hash);
});
addEventListener("popstate", () => void render());

void drawNav();
void render();
