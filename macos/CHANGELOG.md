# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.1.0] — 2026-10-25

The first update since 1.0.0: a usage history, a status line that is yours to
shape, and a long list of fixes, most of them about never letting an account
end up in *Login expired*.

### Security

- **`claude <group>` no longer lets a token in through the environment.**
  `CLAUDE_CODE_OAUTH_TOKEN` and its variants made a session be served by that
  token instead of the account the group activated — while the sensor still
  filed the usage under the profile's e-mail. `CLAUDE_SECURESTORAGE_CONFIG_DIR`
  sent Claude Code to read the credential somewhere else, so a switch had no
  effect. Both are now stripped from every process the router launches, with
  `CLAUDE_CODE_CUSTOM_OAUTH_URL` and `ANTHROPIC_PROFILE`. Found by
  [@viniventur](https://github.com/viniventur) while porting to Windows, in
  [#6](https://github.com/Falkzera/falcao-token-router/pull/6).
- **More of the environment is stripped**: `ANTHROPIC_CONFIG_DIR`, the
  federated identity variables, Foundry, `AWS_BEARER_TOKEN_BEDROCK`, and the
  whole `ANTHROPIC_FOUNDRY_*` and `ANTHROPIC_AWS_*` families, matched by prefix
  so the next variable in a family doesn't slip through
  ([#11](https://github.com/Falkzera/falcao-token-router/issues/11)).
- **No credential is left behind.** An interrupted, failed or duplicate sign-in
  used to leave a live refresh token in the keychain, in a place the app never
  showed; the app now discards it, and sweeps what earlier versions left at
  launch. Deleting a group takes its keychain item and its profile folder
  along (never your `~/.claude`). An unreadable `config.json` is set aside with
  a date and a warning, instead of being overwritten by an empty one.
- **Signing in again as a different account** no longer leaves that other
  account's credential under this account's name, and rotation no longer
  overwrites a sign-in that is still in progress
  ([#12](https://github.com/Falkzera/falcao-token-router/issues/12)).
- **The app no longer decodes the keychain credential at all.** The plan shown
  in the meter now comes from `~/.claude.json`, which carries it with no secret
  next to it.

### Added

- **Usage history.** A **History** tab draws two lines per account — the 5-hour
  window over the last 7 days and the weekly window over the last 4 weeks — from
  the probe's measurements, with the group's threshold dashed on both. While the
  app is open it measures each account every 5 hours (**Meter → History**, on by
  default); **Measure accounts** and `router measure` add their points too. More
  than 7½ hours without a point breaks the line instead of guessing. The points
  are kept for 90 days in `history/<email>.jsonl` — the same file, in the same
  format, that the Windows app writes. Ported from the Windows app by
  [@viniventur](https://github.com/viniventur) in
  [#36](https://github.com/Falkzera/falcao-token-router/pull/36).
- **A shortcut to each account's history**: the chart button next to every
  account on the Groups tab, and **View history** on an account's card in the
  menu bar panel, open the History tab right on that account. **Delete
  history…**, in **Meter → History**, empties it after a confirmation.
- **A signed-out account is named, not just counted.** "Measure accounts" used
  to say only how many accounts didn't answer. The account the probe finds
  signed out (or that "Use" finds with no login on this Mac) now carries a
  yellow "signed out" tag in the Groups list, the menu bar panel and the account
  card; in Groups the tag opens "Log in again". It clears when the account
  answers a measurement, a login is renewed, or a session it serves reports
  usage again. By [@viniventur](https://github.com/viniventur) in
  [#40](https://github.com/Falkzera/falcao-token-router/pull/40).
- **Choose what the status line shows**, in Settings, with a live preview drawn
  by the same code the sessions use. Ten items, one toggle each; the choice is
  kept in `statusline.json` — the same file and format as the Windows app — and
  sessions pick it up on their next update.
- **A Windows app**, in [`windows/`](../windows/README.md) — Rust + Tauri,
  reading and writing the same files as the macOS app. It has its own README,
  CHANGELOG and release tags. Contributed by
  [@viniventur](https://github.com/viniventur) in
  [#8](https://github.com/Falkzera/falcao-token-router/pull/8).

### Changed

- **A group's status line is the full line.** It used to be `account 5h 7d` and
  nothing else — and since the router owns each profile's `statusLine` (the line
  *is* the sensor), it replaced whatever status line you had, including in the
  default group, which is your `~/.claude`. Now it shows

      ● group │ Model effort │ branch │ context │ 5h … ↻ 14:05  7d … ↻ Mon (28) 9:00 │ $cost │ e-mail

  with the group first and the active account's e-mail last, which is where a
  switch shows up. Everything comes from the JSON Claude Code already sends.
- **Removing the account in use hands the group to the next one**, through the
  usual hot swap, so open sessions keep going on it. Before, the group kept
  being served by the removed account and the window showed it with no account
  in use. An account marked signed out is skipped; if nobody can take over, the
  group's item is left alone, so a live session isn't cut off. By
  [@viniventur](https://github.com/viniventur) in
  [#38](https://github.com/Falkzera/falcao-token-router/pull/38).
- **Releases are tagged per platform**: `macos-v*` here, `windows-v*` for the
  Windows app. A bare `v*` tag no longer triggers anything, and GitHub's
  `releases/latest` is ambiguous between the two — links to a download name a
  tag.

### Fixed

- **One account, one place.** The same refresh token living in two keychain
  items dies on the first renewal, and four paths still allowed it: freeing
  `~/.claude` (the account there kept serving plain `claude` and rotation
  activated it again elsewhere), going back to a dedicated profile, an account
  activated in the middle of "Measure accounts", and a switch that failed
  halfway. Each ended in a session stuck on *Login expired*.
- **Rotation no longer gets stuck.** It used to insist on an account it could
  not activate and stay on the exhausted one in silence; a manual "Use" on an
  unmeasured account was undone three minutes later; and a login that isn't a
  router account was swapped out without being copied anywhere.
- **The meter no longer freezes after launch.** It watched the projects folder
  in a way that never saw new lines, so today, week, month and burn rate stayed
  at the values of the moment the app opened.
- **`claude <group>` no longer closes the terminal tab** when the session ends,
  and an old `alias claude` no longer disables the integration in silence (it
  sent `claude work` to the wrong account).
- **"Credential expired" is gone from the meter** when there simply was no
  recent sample: the panel shows the cached number with its age.
- **Prices.** Opus 5.5, Sonnet 5.5 and Fable 5.1 had none, so totals were
  partial; Sonnet 5 was billed 50% too high since September; cache reads are
  priced per model.
- **Numbers that said something else.** A probe failure from a lost connection
  told you to sign in again; the menu bar showed another profile's percentage
  next to an unmeasured account; switching accounts could fire a false "full
  capacity again" alert; the status line showed the main repository's branch
  inside a worktree; yellow started at 66% in the panel and 70% on the line;
  installing the status line replaced a symlinked `settings.json` with a copy.
- **A signed-out account is recognized** when `claude` exits normally with no
  usage lines, which is how it reports a profile with no login.
- **Errors that were swallowed**: a `~/.zshrc` the app can't write now says
  which line to add by hand, and `router rotate` exits with an error and names
  the group that didn't switch.

## [1.0.0] — 2026-09-22

First stable release of the router. Version numbers restart at 1.0.0 because
this is a different product from the token meter it grew out of — the meter
lives on as the *Meter* tab.

### Added

- **Groups of accounts with automatic rotation.** Create a group, sign accounts
  in through Anthropic's official flow inside the app, set the order and the
  threshold. When the active account crosses it, the group switches to the next
  one — inside the live session, no restart.
- **`claude <group>` in the terminal.** One click installs a shell function and
  the status-line sensor in every group profile.
- **Passive sensor.** The status line hands the app the `rate_limits` Claude
  Code already received; the app reads it from stdin and writes one sample per
  account. No network call of its own.
- **Active probe** (*Measure accounts* / `router measure`). Asks the official
  binary via `claude /usage` — the only source that sees the **per-model
  limit**, and the only way to measure an idle account that never served a
  message. Probes an active account through its group profile, never its home.
- **Provenance on every number:** which window (5h, 7d, per-model), which
  source (sensor or probe), how old.
- **Live sessions per group.** Reads Claude Code's per-profile session registry
  to show which sessions run in which group, and whether they're working or
  waiting.
- **`router doctor`** — names what's wrong: integration paths, sensor per
  profile, active account and sample age, sessions, an account active in two
  groups.
- **Show in Dock** setting, and the window opens on first launch.
- English and Brazilian Portuguese.

### Safety rules, enforced by the engine and pinned by tests

- Mirror before swap: the live token goes back to the leaving account's home.
- One account, one place: never active in two groups at once.
- Re-activating the active account never overwrites its rotated token.
- Never probe the home of an active account.
- Removing an account deletes its credential and its home.

### Known limitations

- Ad-hoc signed, not notarized: macOS blocks the first launch (see the README).
- macOS 26+ only. Ports are welcome — see `docs/PORTING.md`.
- Keychain "Always Allow" does not survive an update, because an ad-hoc
  signature changes with every build.

[Unreleased]: https://github.com/Falkzera/falcao-token-router/compare/macos-v1.1.0...HEAD
[1.1.0]: https://github.com/Falkzera/falcao-token-router/releases/tag/macos-v1.1.0
[1.0.0]: https://github.com/Falkzera/falcao-token-router/releases/tag/v1.0.0
