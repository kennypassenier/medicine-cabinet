// The data layer behind `api` (../api.js): the same calls and result shapes as
// the server's /api/mc routes, served from the local database. When the
// browser blocks that database, `useOnlineStore()` swaps every call for its
// plain-fetch twin in ./online.js (same names, same shapes).

import { intakeStore } from "./intakes.js";
import { listStore } from "./lists.js";
import { medicineStore } from "./medicines.js";
import { onlineStore } from "./online.js";

export const store = { ...medicineStore, ...intakeStore, ...listStore };

/** Serve every call over the network from now on (the local database is unavailable). */
export function useOnlineStore() {
  Object.assign(store, onlineStore);
}
