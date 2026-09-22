# How it works

This is the map. Each folder's `agent.md` has the territory — read this first,
then the `agent.md` of wherever you're about to touch.

Almost everything below was discovered by observing Claude Code, not by reading
documentation — none of it is documented by Anthropic. Where a fact was verified
against a real install, the code comment says when. Treat every one of them as
"true on macOS, as of the version noted", and verify before building on it
elsewhere.

## The constraint

Anthropic's terms reserve the OAuth token to the official client. Everything in
this design follows from taking that seriously:

- **The app makes no network calls.** Not one.
- **The app never authenticates.** Sign-in runs the official `claude auth login`
  binary; the app never sees a password or a token.
- **The app never refreshes OAuth.** It copies a keychain *blob* between items.
  It never decodes it to use a token, and `ClaudeCredentials` has no field for a
  refresh token — the guarantee is structural.

The consequence is that usage has to come from somewhere the official client
already put it. There are two such places, and they are the two measurement
paths below.

## The three targets

| Target | Role |
|---|---|
| `CCUsageCore` | The engine. No SwiftUI. Everything testable without a window. |
| `FalcaoTokenRouter` | The SwiftUI app. Menu bar, panel, groups window, settings. Every user-facing string. |
| `router` | The CLI the app bundles at `Contents/MacOS/router`. The shell and the status line call it. |

`CCUsageCore/Engine` is the router product. The rest of `CCUsageCore` — `Aggregation`,
`Models`, `Parsing`, `Pricing` — is the token *meter* this project grew out of,
which still powers the "Meter" tab.

## Profiles, homes and groups

Claude Code keeps everything for one login in a **profile**: a config directory.
The default is `~/.claude`; `CLAUDE_CONFIG_DIR=<path>` selects another. Two
asymmetries matter, both confirmed on disk:

- The profile's `.claude.json` lives **beside** the default directory
  (`~/.claude.json`) but **inside** a dedicated one (`<dir>/.claude.json`).
- Setting `CLAUDE_CONFIG_DIR=~/.claude` explicitly is **not** the same as leaving
  it unset — set, the session comes up logged out. The default profile is
  therefore distinguished by exporting *no* variable. `ConfigDir.isDefault`
  carries that.

The credential for a profile is a keychain item named
`Claude Code-credentials` for the default profile and
`Claude Code-credentials-<sha256(path)[:8]>` for any other — hashed from the
**raw** `CLAUDE_CONFIG_DIR` string, NFC-normalized, not path-resolved. Resolving
a `~` or a trailing slash changes the hash and points at an item that doesn't
exist. The blob has no identity in it; who the credential belongs to is written
in the `.claude.json` next to it (`oauthAccount.emailAddress`).

On top of that, the product defines:

- **Home** — `accounts/<uuid>/` under the app's data directory. Each account
  signs in once, here. Its keychain item is the *mother* credential.
- **Group** — a named, ordered list of accounts and a threshold. Each group has a
  **profile** where its sessions run: the default group uses `~/.claude` (so a
  bare `claude` lands in it), every other group a dedicated `groups/<uuid>/`.

Group profiles symlink `projects/`, `history.jsonl`, `skills/` and friends from
`~/.claude` (`ProfileSharing`), so `--resume` and your tools work in every group.
`settings.json` is deliberately **not** shared: each group's needs its own
`statusLine` pointing at the sensor.

A dedicated profile has to be born with `hasCompletedOnboarding: true` in its
`.claude.json`, or Claude Code opens the first-run wizard without consulting the
keychain at all. `AnthropicAdapter.writeIdentity` does that.

## The swap

Activating an account in a group (`RotationEngine.activate`):

1. Copy the secret from the account's **home** item into the **group's** item.
2. Write the account's identity into the group's `.claude.json`.

That's it. A live session re-reads its keychain item on the next request and is
served by the new account — no restart, no `--resume`, no lost context. This was
proved on a real session before anything else was built.

Two rules make it safe, and both exist because their absence killed accounts:

- **Mirror before you swap.** While an account is active, Claude Code refreshes
  the *group's* item, and the refresh token rotates on every renewal. The home
  falls behind. Before activating anyone else, the fresh token is copied from
  the group item back to the leaving account's home. The home is the truth only
  when the account is idle; `rotateAll` also mirrors periodically so it never
  falls too far behind.
- **One account, one place.** The same account active in two groups would be two
  copies of a rotating refresh token, and the second renewal invalidates the
  first — silently. The engine refuses.

Corollary: re-activating the account that is *already* active must never copy
home → group. That would overwrite the rotated token with a stale one ("Login
expired"). It mirrors the other way. The one legitimate home → group push with
an active account is after a **re-login** (`pushHomeToGroup`): the home has the
new credential and the group holds the dead one that motivated the re-login.

Inside a group session, `/login` is forbidden: it would write into the item the
engine manages.

## Measurement

### The sensor (passive)

Claude Code's status line feature pipes a JSON document to a command on every
turn, and that document includes `rate_limits` — the same headers the API just
returned for the request the account **served**. `router statusline` reads it
from stdin, writes one sample per account under `usage/<email>.json`, and prints
the coloured line the user sees.

It costs nothing and it's exact. It has two blind spots:

- It only sees the 5-hour and 7-day windows. The **per-model** limit is not in
  `rate_limits`, and it's the one that locks an account first.
- It only measures an account that **serves a message**. An idle account has no
  sample at all.

### The probe (active)

`claude --print --no-session-persistence --strict-mcp-config "/usage"` prints
three lines — session, weekly (all models), and weekly per model — off the
credential the CLI already holds. `ClaudeUsageProbe` runs it and parses the text.
The request is the official client's own; the app still touches no endpoint.

Each flag pays its way: `--print` avoids the interactive mode and the
workspace-trust dialog; `--no-session-persistence` avoids writing a transcript
per poll; `--strict-mcp-config` with no `--mcp-config` starts no MCP server —
without it every poll launches whatever the user has configured. Telemetry is
deliberately left on: the per-model line is behind a feature gate that
`DISABLE_TELEMETRY` closes.

It costs a Node cold start and a real request per account, so it's a button and
a command (`router measure`), never a loop. **An active account is probed
through its group profile, never its home** (`probeConfigDir`): probing the home
would renew with the stale refresh token and rotate the chain out from under the
live session.

### Provenance

A sample says which source produced it (`UsageOrigin`: `sensor` / `probe`), and
per-model windows carry their own timestamp because they only move when someone
probes. The UI shows all of it — window, source, age — because the two sources
age differently: a sensor sample on a shared account is optimistic by
construction (what colleagues spent since is invisible), while a probe is a
point-in-time answer.

### The decision

`GroupUsageReader` takes, per account, the **largest** of the three windows
(5h, 7d, tightest per-model) whose reset hasn't passed. A window whose reset has
passed is dropped, not kept — otherwise a stale sample leaves an account "full"
forever. An account with no sample is **presumed fresh**: requiring a sample
created a deadlock (only a serving account gets measured; only a measured
account gets chosen).

`RotationEngine.rotationTarget` switches only when the active account is past
the group's threshold *and* a better destination exists, in preference order.
No destination → stay put. The worst case is "didn't switch", never "stuck".

The loop lives in the app, every 3 minutes: mirror, re-read usage, rotate.
`rate_limits` only changes with activity, so polling faster brings no new number.

## Sessions

Claude Code writes `<profile>/sessions/<pid>.json` for every session — pid,
cwd, session id, process start time, derived name, status (`busy`, `idle`,
`shell`, `waiting`). The registry is **per profile**, which means it answers
what `/status` can't: which session runs in which group, and therefore which
account serves it.

The file outlives the process. `ProcessLiveness` checks the pid **and** the
start time — pids get recycled on a machine that stays up for days, and a
recycled one would resurrect a dead session. An unknown status becomes `.other`,
never `idle`.

## The terminal integration

`claude <group>` is a shell function (`shell.sh`, sourced from `~/.zshrc`) that
asks `router is-group` and, if yes, `exec`s `router launch <group>`, which
activates the right account and `execvp`s the real `claude` with
`CLAUDE_CONFIG_DIR` set (or unset, for the default group). Anything else falls
through to the binary.

The failure mode is silent: in a terminal opened before the integration was
installed, `claude trabalho` is just an argument, and the session opens in
`~/.claude` on the wrong account. The sessions registry is what makes that
visible. `shell.sh` and each profile's `statusLine` embed the router's absolute
path; the app heals both at launch if the `.app` has moved.

`router doctor` checks all of this and names what's wrong.

## What's macOS-specific

| Piece | macOS | Where |
|---|---|---|
| Credential store | Keychain, via `/usr/bin/security` (same binary Claude Code uses — avoids the authorization prompt) | `SecurityCLIKeychain` |
| Process liveness | `sysctl(KERN_PROC_PID)` start time | `ProcessLiveness` |
| Sign-in terminal | `openpty` | `LoginSession` |
| UI | SwiftUI `MenuBarExtra`, `Window`, `LSUIElement`, `SMAppService` | `FalcaoTokenRouter` |
| Shell | zsh function in `~/.zshrc` | `ShellIntegration` |
| Data directory | `~/Library/Application Support/<bundle-id>/` | `RouterPaths` |

Everything else — the config model, the sample format, the rotation rules, the
`/usage` parser, the sessions registry reader — is plain Foundation and reads
files Claude Code writes the same way everywhere. See `PORTING.md`.
