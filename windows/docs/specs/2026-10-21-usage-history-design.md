# Usage history — design

Windows port only. Approved with the maintainer on 2026-10-21.

## What it is

The app measures every account with the probe (`claude --print /usage`) **every 5
hours** while it runs, keeps each measurement, and shows **two line charts per
account** in a new **History** tab: the 5-hour window over the last 7 days, and the
weekly window over the last 4 weeks.

## Decisions taken with the maintainer

| Question | Decision |
|---|---|
| Where the points come from | **Only the probe**: the 5-hour schedule, **Measure accounts** and `router measure`. The sensor's samples do not enter the history. |
| Where the charts live | A **History** tab in the main window, between Groups and Settings. |
| Period of each chart | 5-hour window: **last 7 days**. Weekly window: **last 4 weeks** (28 days). No period control. |
| Who writes the history | **The engine** (`router-core`), inside `ClaudeUsageProbe::measure_into`, in its own folder. The app only decides *when* to measure. |

This reverses a documented rule: the probe was "a button and a command, never a
loop", because each run starts a `claude` and makes one request per account. Every
5 hours is about five runs per account per day, and the schedule can be turned off.

## Data

**Folder and file.** `<base>\history\<email>.jsonl`, next to `usage\`. The file name
uses the same sanitization as `GroupUsageStore::file_name`, with the `.jsonl`
extension. `RouterPaths::history_dir()` names the folder.

**One line per probe measurement**, in the sample's vocabulary — fractions 0–1,
ISO-8601 dates without fractional seconds, camelCase:

```json
{"sampledAt":"2026-10-21T12:00:00Z","fiveHourPercent":0.29,"fiveHourResetsAt":"2026-10-21T14:05:00Z","sevenDayPercent":0.33,"sevenDayResetsAt":"2026-10-27T09:00:00Z","models":[{"name":"Fable","percent":0.12,"resetsAt":"2026-10-27T09:00:00Z"}]}
```

- Type `UsageHistoryPoint`: `sampled_at` (required), `five_hour_percent`,
  `five_hour_resets_at`, `seven_day_percent`, `seven_day_resets_at` (all optional),
  `models` (`Vec<ModelWindow>`, omitted when empty). The per-model windows are kept
  because the probe already has them; no chart uses them yet.
- A window missing from the reading is **absent** in the point, never zero.

**Writing** (`UsageHistory::append(point, email, dir, now)`):

1. Read the existing file; a line that doesn't parse is skipped, the others stay.
2. Drop points older than **90 days** (`now` is a parameter).
3. Add the new point, keep the lines sorted by `sampledAt`.
4. Write the whole file atomically (`write_atomic`: temp + rename), `\n` endings.

Rewriting instead of appending means a crash never leaves half a line, and pruning
happens in the same step. The file stays small: ~5 points a day, ~450 lines in 90
days.

`measure_into` writes the sample (as today) and then the history point, both under
the engine lock it already holds. **A failure writing the history does not fail
the measurement**: the sample in `usage\`, which drives the rotation, is already
on disk.

**Reading.** `UsageHistory::read(email, dir)` returns the points in ascending order
(missing file = empty). `UsageHistory::last_sampled_at(email, dir)` is the newest
`sampledAt`.

**Removing an account** leaves its history file in place, as it leaves
`usage\<email>.json`; the History tab only lists accounts in `config.json`.

Not changed: `usage\<email>.json`, `config.json` (the contract with macOS) and the
sensor.

## The schedule

**Setting.** Settings → **"Measure accounts every 5 hours"**, on by default. Kept in
the app's `settings.json` as `scheduledMeasure` (missing field = on). It only runs
while the app is running; the README says so.

**The rule, per account** (pure, in `router-core/src/engine/measure_schedule.rs`,
the clock as a parameter): an account is **due** when the newest of these is absent
or at least 5 hours old:

- the newest point in its history (whatever produced it: schedule, button, CLI);
- the last scheduled attempt on it **in this run of the app**, success or failure.

The attempt is what keeps a signed-out account from being probed on every pass: it
is tried again 5 hours later. On the first start with this feature, every account is
due, and the history begins.

**When.** The existing loop checks the schedule with the rotation, every 180 s. The
first check is at the **second** rotation pass (3 minutes after the app starts), so
it doesn't compete with everything else starting at Windows sign-in. It is skipped
while:

- the schedule is off;
- a measurement is already running (the same flag as **Measure accounts**);
- a sign-in is in progress (the probe could touch the home being signed in, as the
  rotation could — it waits too);
- `claude` can't be found (silently: there is no user action to report).

**The run.** `RouterConfigStore::scheduled_measure_plans(now, attempts)` returns, in
group order, one `MeasurePlan` per group with only its **due** accounts; an account
in two groups is measured once (in the first group). The app runs the plans one
after another in a worker thread, outside the store's lock, as **Measure accounts**
does:

- the "measuring" flag names each group in turn and is cleared only at the end — a
  click on **Measure accounts** in between can't start a second measurement;
- the group's **Measure accounts** button shows its spinner while that group runs;
- an active account is probed through its group's profile (`probe_targets`, as
  today);
- after each group: the usage is re-read, the tray redrawn, `snapshot-changed`
  emitted;
- every planned account gets an attempt timestamp (the run's start), success or
  failure.

**Failures stay quiet.** A scheduled run does not raise the red error banner — that
banner reports the user's last action. A failed account shows up as a hole in its
chart, and its usage keeps showing its age, as before.

`router measure` keeps its behaviour and now leaves a point in the history too.

## The History tab

```
 Groups   History   Settings
 ──────────────────────────────────────────────
 Each point is a probe measurement (claude /usage).

 work
 ┌────────────────────────────────────────────┐
 │ conta1 · conta1@exemplo.com                │
 │ measured 2h ago · next at 19:05            │
 │                                            │
 │ 5 hours · last 7 days                 29%  │
 │ 100% ┤                                     │
 │      ┤- - - - - - - - - - - - - switch 90% │
 │  50% ┤      ●          ●                   │
 │      │ ●──╱  ╲   ●──●╱  ╲     ●──●         │
 │   0% ┤╱       ●            ●╱    ╲●        │
 │       Wed  Thu  Fri  Sat  Sun  Mon  Tue    │
 │                                            │
 │ Weekly · last 4 weeks                 33%  │
 │ 100% ┤      ╱│       ╱│      ╱│            │
 │   0% ┤──╱───╯ ╰──╱───╯ ╰─╱───╯ ╰─╱         │
 │       Sep 30   Oct 7    Oct 14   Oct 21    │
 └────────────────────────────────────────────┘
```

**The page.** Groups in the list's order; inside each, the accounts in rotation
order, as in the Groups tab. An account in two groups appears in both. No groups:
an invitation to create one in the Groups tab.

**Every number keeps its provenance**, as everywhere in the app: the *window* is
the chart's title, the *source* (the probe) is said once at the top of the page, the
*age* is the account's status line.

**The account card.** Label, e-mail, and a status line:

- "measured 2h ago · next at 19:05" — the age counted from now (the screen's
  clock), the next time written by the backend;
- "next in a moment" when the account is due and the run hasn't reached it;
- "automatic measuring is off" (instead of the next time) when the schedule is off;
- "no measurements yet" when the account has no point.

**The charts.**

- Vertical axis fixed at 0–100% (marks at 0, 50, 100) on every account, so accounts
  compare at a glance.
- Time axis: one mark per day on the 5-hour chart (weekday, in the app's language,
  from the status line's own names: `qua`/`Wed`); one per week on the weekly chart
  (`30/09` in Portuguese, `Sep 30` in English), at local midnight of today and of 7,
  14 and 21 days before.
- A line with dots. The newest value in range is shown at the right of the chart's
  title. A point whose reading lacks that chart's window is left out of that chart
  (not drawn at zero).
- A dashed line at the group's threshold ("switch at 90%"): it's what the rotation
  compares with, and crossing it in either window triggers a switch.
- **Holes:** two consecutive points more than **7 h 30 min** apart (1.5 × the
  interval: app closed, PC asleep) break the line. The chart never draws what
  wasn't measured.
- A point's native tooltip: when it was measured, the percentage, and when that
  window reset — e.g. `ter (21) 14:05 · 29% · reseta 16:00`. Dates and times are
  written by the status line's `reset_when` (5-hour reset: time only; weekly:
  with the day).
- An empty period says so ("No points in this period.").
- Colours from the app's tokens, light and dark themes. Hand-drawn SVG in Svelte,
  like `Gauge` and `MiniBar` — **no new dependency**.
- `role="img"` with an `aria-label` naming the window and the newest value.

**Refreshing.** The tab asks for the history when it opens and on every
`snapshot-changed` (the loop emits it after each measured group).

**Settings.** The switch, with: "Runs `claude /usage` for each account — the same
query as Measure accounts — only while the app is open."

## Backend

`get_history() -> HistoryView`, built from the store (config, active accounts) and
the history files:

```
HistoryView    { scheduleOn, groups: [GroupHistory] }
GroupHistory   { id, name, thresholdPercent, accounts: [AccountHistory] }
AccountHistory { id, label, email,
                 lastMeasuredAt?, nextMeasureAt?, nextMeasureLabel?,
                 fiveHour: Series, sevenDay: Series }
Series         { from, to, ticks: [Tick], segments: [[Point]], latest?: Point }
Tick           { at, label }
Point          { at, fraction, text, label, resetsLabel? }
```

- The series are already cut to their period (7 and 28 days back from now) and
  split at the holes; `text` comes from `UsagePercent` (the percentage is never
  computed in the front); `label`, `resetsLabel`, the ticks and `nextMeasureLabel`
  are written in Rust, in Windows' time zone and the app's language.
- The front only maps time to x and fraction to y.

`set_scheduled_measure(on) -> SettingsView`; `SettingsView` gains `scheduledMeasure`.

`HomeTab` gains `history` (Rust and TypeScript).

## In the browser

`npm run dev`: the mocked backend answers `get_history` and `set_scheduled_measure`
with `@exemplo.com` data, with the scenarios `&historico=cheio|vazio|lacuna|desligado`
(full, empty, a hole, schedule off).

## Strings

A new `history.` prefix in `scripts/check-strings.mjs` (a new surface, as `tray`
was), `home.tab.history`, and the switch's keys under `settings.`. English and
Brazilian Portuguese, same placeholders in both.

## Tests

TDD, like the rest of the port.

- `router-core`
  - History store: points read in order; pruning past 90 days; a broken line is
    skipped and the others kept; a missing file is empty; the file name is the
    `usage\` one with `.jsonl`; dates without fractions.
  - `measure_into` writes the sample **and** the point (fake runner); a history
    that can't be written doesn't fail the measurement; a reading without the
    5-hour line leaves no 5-hour value (not zero).
  - The schedule rule: no point → due; a point under 5 h → not due; a failed
    attempt under 5 h → not due; exactly 5 h → due; an account in two groups
    measured once; group order kept.
- `router-cli`: `router measure` (with `fake-claude`) leaves a history line.
- `app/src-tauri`: the history view — cut to 7 and 28 days, split past 7 h 30 min,
  ticks in both languages, the group's threshold, "next at", the schedule off; the
  setting on by default, also for an old `settings.json` without the field.
- Front: `svelte-check` and `check-strings`.
- By hand: the browser with the mocked backend (both languages and themes, every
  scenario), and the app in the sandbox with `fake-claude` (which answers `/usage`)
  — nothing installed on this machine.
- `scripts\test.ps1` green: fmt, clippy `-D warnings`, tests, front checks.

## Documentation

- `windows/README.md`: the History tab, the switch, `history\` under *Where things
  live*, and "a button, never a loop" → also every 5 hours while the app runs, when
  the switch is on.
- Root `README.md` and `docs/ARCHITECTURE.md`: "never a loop" gets the Windows
  exception.
- `windows/docs/PLATFORM.md`: `history\` is Windows-only, outside the contract with
  macOS, like `statusline.json`.
- `windows/CHANGELOG.md`: an `[Unreleased]` section (the version is decided at
  release).
- The "never the loop" comments in the code (`claude_usage_probe.rs`, `rotation.rs`,
  `measure.rs`) updated with the why and the date; the `agent.md` of every folder
  touched.

## Files

One reason to change per file; none past 600 lines of production code.

| Where | New | Changed |
|---|---|---|
| `router-core` | `engine/usage_history.rs`, `engine/measure_schedule.rs` | `engine/mod.rs`, `lib.rs`, `engine/router_paths.rs`, `usage/claude_usage_probe.rs`, `engine/router_config_store/rotation.rs`, `statusline/view.rs` (weekday names made public) |
| `router-cli` | — | `measure.rs` (comment only) |
| `app/src-tauri` | `history.rs`, `measure_schedule.rs` | `lib.rs`, `state.rs`, `rotation_loop.rs`, `commands.rs`, `settings.rs` |
| `app/src` | `home/HistoryView.svelte`, `home/HistoryCard.svelte`, `lib/LineChart.svelte`, `lib/mock-history.ts` | `home/Home.svelte`, `home/SettingsView.svelte`, `lib/types.ts`, `lib/api.ts`, `lib/mock.ts`, `locales/*.json`, `scripts/check-strings.mjs` |

`snapshot.rs` (564 lines) is not touched.

## Out of scope

- A chart per model, a period control, a configurable interval.
- History from the sensor's samples.
- The macOS app.
- Measuring while the app is closed (no Windows scheduled task).
