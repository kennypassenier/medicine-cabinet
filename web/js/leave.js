// The "unsaved changes" check shared by the router and the medicine form: the
// form registers how many changes it holds, the router asks before it leaves
// by Back/Forward or by a script (links are guarded by the package's own bar).

import { ask } from "./dialogs.js";

/** @type {() => number} */
let count = () => 0;

/** Register (or clear, with null) the function that says how many changes are unsaved. @param {(() => number) | null} fn */
export function setChangeCount(fn) {
  count = fn ?? (() => 0);
}

/** Resolve true when leaving is fine: nothing is unsaved, or the person confirms. */
export async function mayLeave() {
  const n = count();
  if (n === 0) return true;
  return ask({
    title: "Leave without saving?",
    text: `${n} ${n === 1 ? "change is" : "changes are"} not saved yet. If you leave now they are lost.`,
    accept: "Leave without saving",
    cancel: "Stay and keep editing",
    destructive: true,
  });
}
