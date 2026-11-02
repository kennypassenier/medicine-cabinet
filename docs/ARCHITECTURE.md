# Architecture

How the code is put together, so that you know where a change belongs. Every
statement is read from the source; where a number is counted it is a count of
this branch.

## Overview

```
 browser (web/js, kp-themes)                     background tasks (tokio)
   |  fetch /api/mc/...?now=<local time>           reminders::spawn  every 30 s  ----> kit Notifier ----> kyu
   v                                               agenda::spawn     every 15 min ----> HTTP ----------> Almanac
 api/       JSON handlers (transport)                  |  reads/writes via the same
   |                                                   v  AppDb handle
   +--> domain/   pure rules: dates, schedule, plan, expiry, stock, dosing guard
   +--> store/    all SQL; one module per table family
   v
 AppDb (chassis::storage, sqlite file medicine-cabinet.db, migrations in schema.rs)
```

`src/main.rs` only wires: it builds the kit `App`, reads the reminder settings,
opens the app database with the migrations, starts the background tasks,
mounts the API and the embedded web app, and registers the pages for the
navigation. Everything the kit provides (configuration, logging, `/healthz`,
`/metrics`, the admin login, client tokens, the Status/Clients/Storage pages,
self-update, the notifier, the PWA files, the CSP) comes from chassis-rs and is
described in the kit's docs, not here.

## Modules

| Module | Responsibility | Depends on |
|---|---|---|
| `main.rs` | Wiring (89 lines); the five registered pages (`PAGES`) | everything below |
| `config.rs` | `ReminderConfig` (time zone, digest hour) and `hub_from_env` (the Almanac settings) | chassis `Error`, `chrono_tz` |
| `schema.rs` | The six migrations V1 to V6 as SQL strings; `ALL_SQL` for tests | chassis `Migration` |
| `domain/` | Pure rules, no database and no HTTP (it uses the kit's `Error` type only) | nothing else |
| `domain/date.rs` | `Date` arithmetic on `YYYY-MM-DD`, weekday, `parse_minutes` | |
| `domain/medicine.rs` | `MedicineInput` and its validation; `effective_expiry`, `freshness`, `stock_level` | `date` |
| `domain/schedule.rs` | `SlotInput` (time, weekday bitmask, dose), `check_dose` (the as-needed guard) | `date` |
| `domain/plan.rs` | `PlanSlot` to `PlannedDose` with the state upcoming/due/overdue/taken/skipped | |
| `domain/units.rs` | "1 capsule" / "2 capsules" wording | |
| `store/` | The only place with SQL (plus `schema.rs`, and one count query in the debug-only `seed.rs`) | `domain`, rusqlite via the kit |
| `store/medicines.rs`, `slots.rs`, `intakes.rs`, `symptoms.rs`, `categories.rs`, `forms.rs` | CRUD per table family; the stock rules live in `intakes.rs` and `medicines::adjust_stock` | |
| `store/reminder_log.rs`, `agenda_sent.rs` | The two "already sent" ledgers | |
| `store/mod.rs` | `db_err` (sqlite failure to a 502 `Error` that keeps the cause) and `not_found` | |
| `attention.rs` | What needs attention (stock, expiry, course end) from a medicine list | `domain`, `store::medicines::Medicine` |
| `dayplan.rs` | Today's slots paired with the intakes that answered them | `domain::plan`, `store` types |
| `api/` | One file per area: `medicines`, `intakes`, `today`, `search`, `symptoms` (also categories), `forms`; `mod.rs` has the router and the `Clock` extractor | `store`, `domain`, `attention`, `dayplan` |
| `reminders/` | The 30-second loop (`mod.rs`) and the pure decision of what is due (`due.rs`) | `store`, `dayplan`, `attention`, `domain::plan`, kit `Notifier` |
| `agenda/` | The 15-minute calendar sync: `events.rs` (pure: which events are wanted), `client.rs` (HTTP to the hub), `mod.rs` (the plan and the loop) | `store`, `reqwest` |
| `seed.rs` | Demo data; compiled only with `debug_assertions` | `store` |
| `build.rs` | Generates the table of files under `web/` that is compiled into the binary | |
| `tests/kit_smoke.rs` | The kit's smoke test (login, client token, API call, dashboard); owned by `chassis sync` | |

**Flow of a request.** A handler in `api/` extracts `Query<Clock>` (the `now`
string), parses it with `Date::parse` / `parse_minutes`, then runs a closure on
the database: `db.read(|conn| ...)` for reads and `db.write(|tx| ...)` for
writes (a transaction). The closure calls `store` functions, which return
domain structs; the handler returns `Json`. Every failure is a
`chassis::Error` (kind, message, remedy), which the kit turns into
`{"error","remedy"}` and a status (see [API.md](API.md)). Derived judgements
(`freshness`, `stock_level`, `course_over`, `effective_expiry`) are computed in
`store::medicines::read_row` from the domain functions and returned with the
row, so the browser does not repeat the rules.

## The time convention

The server has no time zone of its own for the screens: **the browser sends its
local clock** as `now=YYYY-MM-DDTHH:MM` with every request that depends on
"today" (`web/js/clock.js` `nowParam`, `web/js/api.js` `timed: true`;
`src/domain/date.rs` explains why: the domain stays pure and the server never
guesses a zone). Intake times (`taken_at`, `planned_for`) are stored as local
wall-clock text `YYYY-MM-DDTHH:MM` exactly as the browser sent them, with no
offset. Consequences:

* The API and the screens are consistent with the browser's clock, wherever
  the server is.
* The **background loops cannot ask a browser**, so they use
  `MEDICINE_CABINET_TIMEZONE` (default UTC) to compute the same "today" and
  "minute". If that variable differs from the zone of the browser you use,
  Today and the reminders disagree about what is due now. The README's
  configuration section says to set it.
* Nothing in the code handles a change of zone or daylight saving: a slot is
  a wall-clock time on a date.
* Machine timestamps (`created_at`, `deleted_at`, `sent_at`) are sqlite's
  `datetime('now')`, which is UTC. The 14-day soft-delete window is measured
  with it.

## Data model

`src/schema.rs`, migrations applied once in order (the kit records the version;
the log says `database migrated ... version=6`). Foreign keys are enforced
(a nonexistent `category_id` fails with `FOREIGN KEY constraint failed`).

```
medicine_forms 1---* medicines *---* symptoms *---1 symptom_categories
                       |  (medicine_symptoms)
                       +--* schedule_slots
                       +--* intakes
                       +--* reminder_log

agenda_sent   (no relations; keyed by the calendar external id)
```

| Table | Columns that matter | Notes |
|---|---|---|
| `medicines` | `name`, `purpose`, `form_id`, `strength`, `notes`, `active`, `as_needed`, `min_gap_minutes`, `max_per_day`, `stock` (real), `stock_unit`, `low_stock_at`, `expires_on`, `opened_on`, `shelf_life_days`, `course_ends_on`, `created_at`, `deleted_at` | `deleted_at` (V6) is the soft delete. The text column `form` from V1 is still there and no longer read (V3 copied its values into `medicine_forms`). Dates are `YYYY-MM-DD` text. |
| `schedule_slots` | `medicine_id`, `time_of_day` `HH:MM`, `days` (bitmask, default 127), `dose` (real, default 1) | Rewritten as a set on every save of a medicine. Cascade on medicine delete. |
| `intakes` | `medicine_id`, `taken_at`, `planned_for`, `status` (`taken` or `skipped`, a CHECK), `dose`, `note` | Index `(medicine_id, taken_at)`. No uniqueness: two intakes may answer one slot. |
| `symptoms` | `name` (UNIQUE), `category_id` (ON DELETE SET NULL) | V2 made them first-class: they survive losing their last medicine. Names are lowercased by the code. |
| `medicine_symptoms` | `(medicine_id, symptom_id)` primary key | Both sides cascade. |
| `symptom_categories` | `name` (UNIQUE, NOCASE) | |
| `medicine_forms` | `name` (UNIQUE, NOCASE), `unit` | Nine rows from V3. `medicines.form_id` is ON DELETE SET NULL. |
| `reminder_log` | `(medicine_id, planned_for, kind)` primary key, `sent_at` | `planned_for` is the slot (`YYYY-MM-DDTHH:MM`) or, for the digest, the date. Pruned after 7 days. |
| `agenda_sent` | `external_id` primary key, `fingerprint`, `sent_at` | What the hub was last told. |

**Stock is a number, not a ledger.** `medicines.stock` is changed by
`adjust_stock` (`MAX(0, stock + delta)`) from four places: a taken intake
(`-dose`), editing an intake (`old - new`), removing one (`+dose`), and the
stock endpoint. There is no history of stock changes.

**Soft delete.** `medicines::delete` sets `deleted_at`; every query that lists
or fetches a medicine, and the intake list, filter on `deleted_at IS NULL`.
`purge_deleted` removes rows older than `PURGE_AFTER_DAYS` = 14 and the
cascades remove everything that hung on them. It is called from the reminder
loop. Proven by `store::tests::a_deleted_medicine_hides_and_comes_back_with_its_history`
and `purge_only_removes_what_is_past_the_window`.

## The reminder loop (`src/reminders/`)

`reminders::spawn` starts a task that sleeps 30 s and runs `tick` forever. A
tick (`mod.rs`):

1. Converts `Utc::now()` to `MEDICINE_CABINET_TIMEZONE`: the local date, the
   minute of the day, and `digest_due` = local hour >= the digest hour.
2. Opens **one write transaction** (`collect`): lists the medicines, today's
   intakes (limit 500) and what `reminder_log` already holds for today;
   builds the day plan (`dayplan::day_slots` + `domain::plan::plan`).
3. Asks the pure functions in `due.rs` what is due: `dose_reminders` (state
   `Due` gives `dose.due`; state `Overdue` and not more than 180 minutes late
   gives `dose.overdue`) and, when `digest_due`, `attention_reminders`
   (`cabinet.attention`, one per attention item).
4. For each candidate runs `reminder_log::mark` (an `INSERT OR IGNORE` on the
   primary key); only the ones that actually inserted are kept. That is the
   dedup, and it is also why two overlapping ticks cannot both send.
5. Prunes log rows older than 7 days and purges medicines deleted more than 14
   days ago, still inside the transaction.
6. After the commit, `notifier.emit(kind, version, text)` for each kept
   reminder. With no notify target the kit only logs the event.

Errors: a tick that fails prints `reminders: this tick failed: ...` to stderr
and the loop goes on; an error whose message contains `not open yet` (the
database is still starting) is ignored silently. Proven by the unit tests in
`reminders/due.rs`: `a_due_dose_is_reminded_once`,
`upcoming_taken_and_ancient_doses_stay_quiet`,
`a_recently_overdue_dose_gets_one_nudge`, `the_digest_skips_what_was_sent`;
and by running the binary twice on one state (the second run sent nothing).

## The calendar loop (`src/agenda/`)

Started only when `config::hub_from_env()` finds a URL and a token.
`agenda::spawn` builds the HTTP client (15 s timeout) and loops: sleep 20 s,
run a `pass`, sleep 15 minutes. A pass:

1. Reads the medicines and the whole `agenda_sent` table.
2. `events::wanted_events` (pure) lists the wanted events: for each *active*
   medicine an expiry event, a course-end event and a reorder event, each only
   when its date is 0 to 120 days away. The id is `expiry:<id>`, `course:<id>`
   or `reorder:<id>`.
3. `agenda::plan` compares with the ledger: **upsert** what is new or whose
   fingerprint (`title|start|description`) changed, **delete** what is in the
   ledger but no longer wanted.
4. Does the HTTP calls one by one, updating the ledger after each success. A
   failure returns early (`unreachable`: "The calendar hub did not accept
   ...", kind dependency) and is printed as `agenda sync: this pass failed:
   ...`; the next pass retries what is left.

Proven by `agenda::tests::only_changed_events_are_sent_and_stale_ones_deleted`
and, in `agenda/events.rs`, `stock_prediction_follows_the_schedule`,
`reorder_event_sits_a_week_before_the_run_out`,
`expiry_and_course_end_become_events_inside_the_horizon` and
`switched_off_as_needed_and_empty_medicines_plan_nothing`. The HTTP shapes
(`POST <base>/v1/ingest/<source>`, `DELETE .../events/<id>`, bearer token) have
no automated test; I checked them against a stand-in HTTP server.

## Frontend module map (`web/`)

Plain ES modules, no build step and no bundler: `build.rs` embeds every file
under `web/` into the binary and the kit serves them (behind the login) with
the vendored kp-themes bundle at `/static/kp/dist/kp-themes.min.js`. The
server answers `index.html` for the app's paths, so a deep link such as
`/medicine/3` works. The JavaScript carries JSDoc type annotations; there is no TypeScript
step, no `package.json` and no frontend test suite in this repository.

| File | Role |
|---|---|
| `index.html`, `css/app.css` | The shell page (top bar, `<main id="page">`, phone tab bar) and the few styles the app adds |
| `js/main.js` | Router and page lifecycle: `navigate`, history API, `mount(root, ctx)` returning a cleanup, "Page not found" plate, theme picker, link interception |
| `js/routes.js` | One list of places (`AREAS`, `MORE`) that drives the bar, the tab bar and `route()`; `/find` redirects to `/symptoms` |
| `js/shell.js` | Draws the bar, the tab bar and the More menu (Settings plus the kit's Status, Clients, Storage from `/api/kit/pages`) |
| `js/api.js` | The only module that calls `/api/mc`: adds `now`, turns failures into `ApiError(message, remedy, status)`; plain-text rejections are shown as they are |
| `js/clock.js` | Local date/time in the API's shapes; `dd/mm/yyyy` display, `Today/Yesterday`, durations, weekday labels |
| `js/dom.js`, `ui.js`, `controls.js`, `dialogs.js`, `format.js`, `units.js` | Helpers: element builder `h` and `redraw` (keeps focus and caret), page header and data card with loading/empty/error/ready states and toasts, stepper/switch/chips builders, confirmation dialog, quantity and badge wording, unit suggestions |
| `js/inline.js`, `entity-table.js` | In-place editing fields, add rows, row menus; the sortable table used by Settings and Symptoms |
| `js/undo.js`, `leave.js`, `ticker.js` | The 8-second Undo toast; the unsaved-changes check before leaving; refresh on the kit's tick, on focus and after an Undo |
| `js/dose.js`, `stock.js` | The shared as-needed flow (ask the guard, explain, "Take anyway", log, Undo) and the Add stock popover |
| `js/pages/today*.js` | Today: `today.js` (state and loading), `today-doses.js` (next dose, schedule), `today-attention.js`, `today-asneeded.js` |
| `js/pages/cabinet*.js` | Cabinet: list, filters (`cabinet-filters.js`), row (`cabinet-row.js`), debounced stock editor (`cabinet-stock.js`) |
| `js/pages/medicine-form.js`, `form-*.js` | The add/edit form: body and model (`form-body.js`), cards (`form-sections.js`, `form-when.js`, `form-schedule.js`, `form-symptoms.js`, `form-pills.js`), field builders, the mirror fields that make the schedule visible to the unsaved bar |
| `js/pages/symptoms*.js`, `symptom-manage.js`, `symptom-categories.js` | The symptom search and the management of symptoms and categories |
| `js/pages/history*.js` | The intake log, in-place edit rows, "Add an earlier dose" |
| `js/pages/settings.js` | Medicine forms |

Rules the front end keeps: the server is the source of truth and nothing is
kept in browser storage; every reversible action runs at once and offers Undo
instead of asking first; a failed call is shown to the person (`reportError` toast, or the error state
of the card) with the server's message and remedy; the only quiet fallbacks
are in `api.js` (a body that is not JSON is shown as text) and `shell.js` (if
the kit's page list cannot be read, the More menu shows only Settings, with a
console warning). Screens are built from kp-themes parts.

## Installable app

`main.rs` calls `app.webapp(WebApp::embedded(FILES))` with no `.pwa(...)`
call, so the kit's defaults apply: it serves `/manifest.webmanifest`
(name `Medicine cabinet`, short name `Medicine cab`, standalone, theme colour
`#223659`), `/sw.js` and `/pwa/offline`, and adds the manifest, theme-colour
and `apple-touch-icon` tags to the page. The icons come from `web/pwa/`.

Observed with the pinned kit (`Cargo.toml`, rev `3cbae24`, version 4.1.0): the
manifest lists `icon-192.png`, `icon-512.png` and `icon-maskable-512.png`;
`/pwa/icon-180.png` is served for the iOS tag. `web/pwa/` also holds
`icon-maskable-192.png` and `icon-monochrome-512.png` (added in the latest
commits); this kit version does not serve them (they answered `303` to the
login) and does not list them. The commit message says a newer kit lists every
file the app supplies. `assets/icon.svg` and `icon-maskable.svg` are the
sources and are not served.

## Engineering rules, and how well the code keeps them

The rules are Kenny's `ENGINEERING_RULES.md` (2026-10-30). Measured on this
branch:

* **Files up to about 250 lines, one responsibility.** Longest Rust file:
  `src/store/medicines.rs` with 249 lines; then `seed.rs` 217, `store/tests.rs`
  207. Longest JavaScript file: `pages/symptom-manage.js` with 163 lines. All
  77 Rust and JavaScript files under `src/` and `web/js/` are under 250.
* **Functions under about 40 lines.** Mostly kept. Over 40 lines in Rust:
  `seed::demo_medicines` (69, a data table), `api::router` (49, a route
  table), `main` (43, the wiring), `domain::schedule::check_dose` (42),
  `attention::attention_for` (40). In JavaScript the longest functions are
  `createWhen` (39) and `addEarlierDose` (38); a commit
  (`40217f5`) split every JS function over 40 lines.
* **Separation of config, domain, persistence, transport.** `config.rs`,
  `domain/`, `store/` and `api/` as in the table above. The pure parts
  (`domain/`, `reminders/due.rs`, `agenda/events.rs`, `attention.rs`) have no
  I/O and carry the unit tests; the loops are thin shells around them.
* **No silent errors.** Errors carry a message and a remedy and reach the
  user or the log. Two places swallow detail: `intakes::record` turns any
  failure of its existence query into "medicine N does not exist"
  (`.unwrap_or(false)`), and `dayplan` falls back to minute 0 for a slot time
  that does not parse (the API validates slot times, so only damaged rows get
  there). The reminder and agenda loops print failures to stderr rather than
  surfacing them anywhere in the app.
* **No hard-coded configuration.** The time zone, digest hour and calendar
  settings come from the environment. The behaviour thresholds are named
  constants in the code and are **not** configurable: due window 60 minutes
  (`plan::DUE_WINDOW_MINUTES`), overdue nudge limit 180
  (`due::OVERDUE_LIMIT_MINUTES`), "expiring soon" 30 days (`SOON_DAYS`), course
  warning 0 to 3 days (`attention.rs`), purge after 14 days, reminder tick
  30 s, calendar pass 15 min, horizon 120 days, reorder lead 7 days.
* **Tests.** 38 Rust unit tests in `src/` (config 3, date 5, medicine 4,
  schedule 5, plan 3, units 2, due 4, agenda 1 + events 4, store against
  in-memory sqlite with the real migrations 7) plus the kit smoke test.
  The API handlers, the reminder tick and the HTTP client have no automated
  test of their own; the front end has none.

Gates (`.githooks/pre-commit` calling `.claude/hooks/gates.sh`): `cargo fmt
--check`, `cargo clippy --all-targets -D warnings`, `cargo test` (skipped when
no Rust source moved since the last green run), and a check that the commit did
not change the tree it tested.
