# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- **Releases are tagged per platform.** The macOS app releases on `macos-v*` and
  the Windows port on `windows-v*`; a fix on one no longer waits for the other's
  calendar. `Scripts/release.sh` creates `macos-v<VERSION>`.

  Two consequences worth knowing. A bare `v*` tag **no longer triggers anything**
  — the `v1.0.0` release of 2026-09-22 stays published, but a `v1.0.1` pushed
  today would go by in silence. And GitHub's `releases/latest` is now
  **ambiguous**, because it resolves to whichever platform released last: links
  to a download have to name a tag.

### Added

- **A Windows port**, in [`windows/`](windows/README.md) — Rust + Tauri, reading
  and writing the same files as the macOS app. It has its own README, CHANGELOG
  and release tags. Contributed by [@viniventur](https://github.com/viniventur)
  in [#8](https://github.com/Falkzera/falcao-token-router/pull/8).

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

[1.0.0]: https://github.com/Falkzera/falcao-token-router/releases/tag/v1.0.0
