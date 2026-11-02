// The app shell: top bar, phone tab bar, theme picker and the history-API
// router. Each screen is a module with `mount(root, ctx)` returning its
// cleanup. The server answers index.html for every extension-less path, so
// a deep link (/medicine/3) lands here too.

import { attachThemePickers, themeMenuMarkup } from "/static/kp/dist/kp-themes.min.js";
import { attachSegmented, attachAttention, attachSkipLinks, applyStoredCalm } from "/static/kp/dist/kp-themes.min.js";
import { h } from "./dom.js";
import { route } from "./routes.js";
import { linkButton, pageHeader, reportError } from "./ui.js";
import { drawShell, markNav } from "./shell.js";
import { mayLeave } from "./leave.js";

const page = /** @type {HTMLElement} */ (document.getElementById("page"));
const tools = /** @type {HTMLElement} */ (document.getElementById("tools"));

applyStoredCalm();

/** @type {() => void} */
let cleanup = () => {};
let seq = 0;
/** The first screen is read from the top by itself; every later one moves the focus to its title. */
let firstScreen = true;

/** After a route change: the focus goes to the page title (it is read out), the top of the page when it has none yet. */
function focusTitle() {
  const target = document.getElementById("page-title") ?? page;
  target.focus({ preventScroll: true });
}

/** @param {string} href */
export async function navigate(href) {
  if (!(await mayLeave())) return;
  const url = new URL(href, location.href);
  if (url.origin !== location.origin || !route(trimPath(url.pathname), url.search)) return void location.assign(href);
  history.pushState(null, "", href);
  shown = location.href;
  void render();
}

/** The address the screen on show belongs to: Back/Forward restores it when leaving is refused. */
let shown = location.href;

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
  shown = location.href;
  if (!found) {
    document.title = "Not found · Medicine cabinet";
    page.replaceChildren(plate("Page not found", "There is nothing at this address. Check the address, or go back to Today.", "Back to today", "/"));
    return void (firstScreen ? (firstScreen = false) : focusTitle());
  }
  let mod;
  try {
    mod = await found.load();
  } catch (e) {
    if (mine !== seq) return;
    reportError("This screen could not be loaded.", e);
    page.replaceChildren(plate("This screen could not be loaded", "Reload the page to try again. If it keeps failing, the service may be restarting; try again in a minute.", "Reload", location.href));
    return void focusTitle();
  }
  if (mine !== seq) return;
  page.replaceChildren();
  const done = mod.mount(page, { params: found.params, navigate });
  if (typeof done === "function") cleanup = done;
  if (!location.hash) window.scrollTo({ top: 0 });
  if (!firstScreen) focusTitle();
  firstScreen = false;
}

/** The page for a screen that cannot be drawn at all: a page header (the one h1) with the way out. */
function plate(title, text, label, href) {
  return pageHeader({ title, description: text, actions: [linkButton(label, label === "Reload" ? location.pathname : href, "kp-button--primary")] });
}

const theme = h("span", { class: "mc-theme" });
theme.innerHTML = themeMenuMarkup({ id: "mc-theme-menu", label: "Theme" });
tools.replaceChildren(theme);
attachThemePickers(tools);
attachSkipLinks(document);
// Chips and pills are drawn by the screens after load; one listener on the document wires every group.
attachSegmented(document);
// The attention band (its cap and "Show N more") is built by the Today screen after load.
attachAttention(document);

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
addEventListener("popstate", async () => {
  if (location.href === shown) return;
  if (await mayLeave()) return void render();
  history.pushState(null, "", shown);
});

void drawShell(route(trimPath(location.pathname), location.search)?.bar ?? "");
void render();
