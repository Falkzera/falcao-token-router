# Changelog — Windows

The Windows app versions on its own, with `windows-v*` tags. The macOS app has
its own [`macos/CHANGELOG.md`](../macos/CHANGELOG.md) and its own `macos-v*` tags:
a fix on one platform never waits for the other's calendar.

> ⚠️ `releases/latest` is **ambiguous** in this repository — it resolves to
> whichever platform released last. Link to a tag, never to `/latest`.

## [Unreleased]

## [1.1.0] — 2026-10-25

A usage history, a tag on the account that needs signing in again, and the
fixes from the first month of use — one of them for a freeze of the whole app.

### Added

- **Usage history.** A **History** tab draws two lines per account — the 5-hour
  window over the last 7 days and the weekly window over the last 4 weeks — from
  the probe's measurements, with the group's threshold dashed on both. While the
  app is open it measures each account every 5 hours (**Settings → History**, on
  by default); **Measure accounts** and `router measure` add their points too.
  More than 7½ hours without a point breaks the line instead of guessing. Hover a
  chart, or focus it and use ← →, to read a point. The points are kept for 90 days
  in `%LOCALAPPDATA%\com.synqo.falcao-router\history` — the same files the macOS
  app writes.
- **A shortcut to each account's history**: the chart button next to every account
  on the Groups tab, and **View history** on an account's card in the tray panel,
  open the History tab right on that account. **Delete history…**, in
  **Settings → History**, empties it after a confirmation.
- **A signed-out account is named, not just counted.** "Measure accounts" used
  to say only how many accounts didn't answer. The account the probe finds
  signed out (or that "Use" finds with no login on this computer) now carries a
  yellow "signed out" tag in the Groups window, the flyout and the account
  card; in the Groups window the tag opens "Log in again". It clears when the
  account answers a measurement, a login is renewed, or a session it serves
  reports usage again. The every-5-hours measurement sets and clears it too.

### Changed

- **Removing the account in use hands the group to the next one**, through the
  usual hot swap, so open sessions keep going on it. Before, the group kept
  being served by the removed account and the window showed it with no account
  in use. An account marked signed out is skipped; if nobody can take over, the
  group's profile is left alone, so a live session isn't cut off.

### Fixed

- **Opening Groups or Settings from the flyout could freeze the whole app**,
  tray included, when the Groups window had been closed. The window is now
  created the way WebView2 requires.
- **One account, one place.** The same refresh token in two profiles dies on
  the first renewal, and a few paths still allowed it: freeing `~\.claude`,
  going back to a dedicated profile, and a switch that saved the credential but
  not the identity. Each ended in a session stuck on *Login expired*.
- **Rotation no longer gets stuck** on an account it can't activate, no longer
  undoes a manual "Use" on an unmeasured account, and no longer swaps out a
  login that isn't a router account. It also waits while "Measure accounts"
  runs, which could otherwise probe an account through its home right after it
  became active.
- **A forgotten "Add account" window no longer turns rotation off** for every
  group: only signing an existing account in again holds it.
- **A user name with an apostrophe** (`D'Angelo`), an ampersand or a parenthesis
  broke the status line command, and with it the sensor.
- **An `alias claude` in `.bashrc`** (older Claude Code installers left one)
  disabled the Git Bash integration in silence, so `claude work` opened plain
  `claude` on the wrong account.
- **A measurement without a network connection no longer marks every account as
  signed out**: only an answer that talks about signing in does.
- **Yellow starts at 66%** on the status line too, like the tray.

## [1.0.0] — 2026-09-24

First Windows release. The port reads and writes the same files as the macOS
app, so a group and its accounts mean the same thing on both.

### The app

- **Groups of accounts that take turns.** Create groups, sign accounts in
  through the official flow inside the app, set the order of preference and the
  threshold. The engine swaps the active account on its own when the threshold
  is hit, without ending the session.
- **The tray**, with the ring drawn at the taskbar's small-icon size and in its
  theme. Its tooltip gives, per group, the account, the window, the percentage,
  where the number came from and how old it is.
- **A flyout** with the accounts table, next to the icon, also from the Windows
  11 overflow.
- **The Groups window**: create, rename, default or not, delete, auto-switch,
  threshold, reorder by drag or keyboard, use, sign in again, remove, and
  measure accounts. Every usage number carries its window, its reset time, its
  source and its age.
- **Terminal integration per shell** — PowerShell 7, Windows PowerShell 5.1 and
  Git Bash — each problem shipped with its fix: an execution policy that blocks
  the profile, a `.bash_profile` that ignores `.bashrc`, a `claude` function you
  already had (chained, not replaced).
- **A complete status line** in a group's sessions by default —
  `● group │ model │ branch │ context │ 5h … │ 7d … │ $cost │ e-mail` — with
  every item switchable in **Settings → Status line**, a live preview, or your
  own status-line command running after the sensor. The sensor behind it is the
  same in every mode.
- **`router.exe`** with `statusline`, `launch`, `is-group`, `rotate`, `measure`
  and `doctor`.

### The installer

NSIS, per user, in `%LOCALAPPDATA%\FalcaoTokenRouter`, no administrator rights,
English and Brazilian Portuguese, with the WebView2 bootstrapper. It is **not
code-signed**, so SmartScreen stops it once: *More info → Run anyway*.

An open `claude <group>` session keeps a `router.exe` running, and Windows will
not overwrite a running executable — the installer moves it out of the way
instead, so an update over a live session works.

### What it does not do

The inherited token meter (JSONL cost, pricing, alerts), code signing and arm64
are out of scope for this first release.

[Unreleased]: https://github.com/Falkzera/falcao-token-router/compare/windows-v1.1.0...HEAD
[1.1.0]: https://github.com/Falkzera/falcao-token-router/releases/tag/windows-v1.1.0
[1.0.0]: https://github.com/Falkzera/falcao-token-router/releases/tag/windows-v1.0.0
