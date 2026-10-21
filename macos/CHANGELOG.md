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

## [Unreleased]

### Changed

- **A status line de um grupo passa a ser a linha completa.** Antes o router
  escrevia `conta 5h 7d` e nada mais. Como ele é dono da `statusLine` do perfil
  — a linha **é** o sensor —, ele a escrevia por cima da que você tivesse,
  inclusive no grupo padrão, que é o seu `~/.claude`: ativar a integração de
  terminal custava sua status line. Agora sai

      ● grupo │ Modelo effort │ branch │ contexto │ 5h … ↻ 14:05  7d … ↻ seg (28) 9:00 │ $custo │ e-mail

  com o grupo na frente e o e-mail da conta ativa no fim, que é onde a troca de
  conta aparece. Tudo vem do JSON que o Claude Code já entrega; o branch é lido
  do `.git/HEAD`, sem gastar um processo por render. Escolher quais itens
  aparecem — ou rodar o seu próprio comando depois do sensor — vem em seguida.

  Encontrado pelo porte Windows no primeiro teste real, e portado dele.

### Added

- **Usage history.** A **History** tab draws two lines per account — the 5-hour
  window over the last 7 days and the weekly window over the last 4 weeks — from
  the probe's measurements, with the group's threshold dashed on both. While the
  app is open it measures each account every 5 hours (**Meter → History**, on by
  default); **Measure accounts** and `router measure` add their points too. More
  than 7½ hours without a point breaks the line instead of guessing. The points
  are kept for 90 days in `history/<email>.jsonl` — the same file, in the same
  format, that the Windows app writes. Ported from the Windows app
  ([#35](https://github.com/Falkzera/falcao-token-router/pull/35)).
- **A shortcut to each account's history**: the chart button next to every
  account on the Groups tab, and **View history** on an account's card in the
  menu bar panel, open the History tab right on that account.
- **Delete history…** in **Meter → History** empties the history of every
  account, after a confirmation. Measuring goes on, and the history starts over
  with the next measurement.

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
