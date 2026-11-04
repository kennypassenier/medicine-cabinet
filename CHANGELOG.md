# Changelog

All notable changes to medicine-cabinet. Semantic versioning: a breaking change
in the API, the configuration knobs or the on-disk state is a major, and
carries a **Migration** section saying what an operator must do.

## [Unreleased]

### Added
- **Offline use.** The pages read and write through the kit's local database
  (`chassisData`, chassis-rs 4.7.0): Today, Cabinet, Symptoms, History and
  Settings render after a full reload without a network, a dose taken
  offline is queued and sent once, and an edit the server refuses lands in the
  kit's failed-edits list (the server always wins). Today, the dose guard,
  stock levels and the symptom counts are computed in the page (`web/js/plan/`,
  `web/js/store/`).
- Creating a medicine, symptom, category or form works offline: the page makes
  the id (a UUID) and the create is queued with an `Idempotency-Key`.
- Sync feed: `GET /api/sync` with five collections (medicines, intakes,
  symptoms, categories, forms), written in the same transaction as every change.
- Merged page endpoints `GET /api/mc/pages/{today,cabinet,symptoms,history,settings}`
  (one call per page; every individual resource keeps its own endpoint). The
  pages use them when the browser blocks its local database, with a notice
  "Working online only".
- Soft delete with 14 days of undo, undo toasts, an unsaved-changes bar,
  inline lists and a stock popover, a phone tab bar with four tabs and More.

### Changed
- **Breaking (API and sync feed):** medicines, symptoms, categories and forms
  are identified by a `uid` (UUID text), shown as the JSON field `id`, so they
  can be created offline; every reference to them is a uid too (`form_id`,
  `category_id`, `medicine_id`, `medicine_ids`, `symptom_ids`) and so are the
  route parameters. Creating one accepts an optional `id` and an
  `Idempotency-Key` and answers `{id, rev}`. **Migration:** automatic at start
  (V10 backfills a uid per row; the sync change log is re-keyed). A device
  drops its old local copies and syncs again; edits queued under an integer id
  are refused and land in its failed list.
- Stock is stored as exact hundredths (migration 9); JSON still shows decimals.
- Logged doses stay in History after a medicine is purged: each dose keeps the
  medicine's name (migration 9).
- A medicine edit is stale only when its description changed, not when a dose
  changed its stock (migration 11); a stale edit keeps the stock the server holds.
- Built on chassis-rs 4.7.0 and kp-themes 12.6.0; a page loads in 0.4 s locally
  (was 3.4 s on 4.6.0).

### Fixed
- A reminder is marked sent only after it was emitted, so a crash in between
  sends one twice at worst, never zero times.
- An overdue dose past midnight now gets its reminder (the loop also reads
  yesterday's plan for the last 180 minutes).
- Offline stock follows queued edits and deletes of logged doses; offline
  counts follow links made offline.

### Migration
- Nothing to do for the operator: the database migrates at start (versions 8 to
  11: `rev`/`uid` columns and the sync change log, exact stock, kept history,
  `desc_rev`), existing data stays.
- The ids of medicines, symptoms, categories and forms in the API and the sync
  feed change from integers to uid strings. Anything outside the app that
  stored or sent the old integers must switch to the `id` values the API now
  returns (there is no known such client). A device with edits queued under an
  old integer id gets them refused (400) and sees them in the kit's
  failed-edits list; open the app online once so it syncs again.
- Build and run on chassis-rs 4.7.0 (kp-themes 12.6.0); nothing to configure.

### Known limits
- The undo of a deleted category or form is a new create.
- The queue retries every 30 seconds after a lost connection.
- Dense touch targets on a phone (time button 44 by 42 px, weekday chips 39
  px wide): parked by Kenny in kp-themes.
- kp-themes open items: sideways-scrolling chips (worked around in `app.css`),
  no keyboard path to the undo toast, one accessible name for every row of
  the row editor, no add-row pattern (built with `kp-autogrid`).

## [1.0.0] - 2026-10-31

### Added
- Medicines with purpose, form, strength, notes, expiry and opened-on dates,
  a course end date, and stock with a low-stock threshold.
- Schedules (time, weekdays, dose) and as-needed medicines with a minimum gap
  and a daily maximum; switching a medicine off switches its schedule off.
- Today screen: next dose, take, skip and undo, attention band, as-needed
  quick row with a dosing guard.
- Symptoms with categories, search by symptom ("what do I have for a
  headache"), and CRUD for symptoms, categories and medicine forms.
- Intake history with undo that restores stock.
- Reminders through the kit notifier (`dose.due`, `dose.overdue`,
  `cabinet.attention`) with a sent-log so a restart repeats nothing;
  `MEDICINE_CABINET_TIMEZONE` (default UTC) and `MEDICINE_CABINET_DIGEST_HOUR`
  (default 9).
- Calendar sync with Almanac (optional: `MEDICINE_CABINET_ALMANAC_URL`,
  `MEDICINE_CABINET_ALMANAC_TOKEN`, `MEDICINE_CABINET_ALMANAC_SOURCE`): all-day
  events for an expiry, the last day of a course and a reorder date a week
  before the predicted run-out; only changes are sent, stale events deleted.
- Installable app icons (192, 512, maskable) for the PWA support of chassis.
- Built on chassis-rs 4.0.0 and kp-themes 12.0.0 (the single bundle, the kit's
  runtime and PWA support). Reminders reach kyu through the kit's shared
  sender: set `MEDICINE_CABINET_NOTIFY_URL` and `MEDICINE_CABINET_NOTIFY_TOKEN`.
- Project created with `chassis new`.
