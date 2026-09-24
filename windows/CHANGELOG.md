# Changelog — Windows

The Windows port versions on its own, with `windows-v*` tags. The macOS app has
its own `CHANGELOG.md` at the repository root and its own `macos-v*` tags:
a fix on one platform never waits for the other's calendar.

> ⚠️ `releases/latest` is **ambiguous** in this repository — it resolves to
> whichever platform released last. Link to a tag, never to `/latest`.

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

[1.0.0]: https://github.com/Falkzera/falcao-token-router/releases/tag/windows-v1.0.0
