// The data layer behind `api` (../api.js): the same calls and result shapes as
// the server's /api/mc routes, served from the local database.

import { intakeStore } from "./intakes.js";
import { listStore } from "./lists.js";
import { medicineStore } from "./medicines.js";

export const store = { ...medicineStore, ...intakeStore, ...listStore };
