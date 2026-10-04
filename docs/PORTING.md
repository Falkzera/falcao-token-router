# Porting to a new platform

Falcão Token Router ships on macOS and Windows. A third platform — Linux is the
obvious one — is welcome, and this document exists to make it tractable: it lists
what a port must replace, what it can keep, and what it must verify before
trusting. The maintainers can't test on your platform, so a port is yours to own.

The Windows port is the worked example of answering this document:
[`windows/docs/PLATFORM.md`](../windows/docs/PLATFORM.md) records every Windows
fact it relies on and how each one was checked. Read it alongside this.

Open a **port** issue first so the work is visible. Use a `port/<platform>`
branch. The review will be about the [invariants](../CONTRIBUTING.md#the-invariants),
which hold on every platform, not about the platform itself.

## Two ways to do it

**Reuse a core.** `CCUsageCore` (Swift) is Foundation plus a few Apple
frameworks — CryptoKit for the sha256 that names a keychain item, Security,
CoreGraphics for the gauge geometry — and some Darwin calls, and Swift runs on
Linux and Windows. The platform seams are small: a `KeychainStore`
implementation, `ProcessLiveness`, the sign-in driver, the shell integration
text, and the data directory. Replace those, keep the engine, tests and CLI, and
write a native UI on top. `router-core` (Rust) is the other candidate: it is
written against Windows APIs today, but its seams are the same.

**Rewrite.** Any language, reading and writing the same files. The contracts are
the file formats and the CLI behaviour, all described in `ARCHITECTURE.md`. This
is what the Windows port did — Rust and Tauri, reading and writing the same
files as the macOS app.

Either way, the sensor is the smallest and most portable piece — a program that
reads JSON from stdin and writes one file. Start there: it proves the status
line delivers `rate_limits` on your platform, which everything else depends on.
Two things the Windows port learned there: the **first render of a session has no
`rate_limits`** (write nothing then, or you erase the last reading), and **stdin
may never close** (parse the first complete JSON value, with a deadline).

## What to verify first

Do this before writing any code. Each is a fact about Claude Code on **your**
platform, and neither existing answer may transfer.

| Question | macOS | Windows | How to check |
|---|---|---|---|
| Where does a profile keep its credential? | Keychain item `Claude Code-credentials[-<hash>]` | A file, `<profile>\.credentials.json` | Sign in with `CLAUDE_CONFIG_DIR=/tmp/p claude auth login`, then look for a `.credentials.json` inside the profile, or a system secret store entry |
| Does a live session pick up a swapped credential? | Yes, on the next request | Yes — but only when the file's **modification time** changes | Swap it under a running session and watch the status line's `resets_at` |
| Does `.claude.json` live beside the default profile and inside a dedicated one? | Yes | Yes | `ls -la ~ ~/.claude /tmp/p` after signing in to each |
| Does the status line receive `rate_limits`? | Yes | Yes, except on the first render | Set `statusLine` to a script that dumps stdin to a file; send one message |
| Is `<profile>/sessions/<pid>.json` written? | Yes | Yes; `procStart` is a FILETIME | Open a session, `ls <profile>/sessions/`. The record's `pidDomain` field names the platform |
| What does `claude --print "/usage"` print? | Three `Current …` lines, `resets Sep 22 at 8:40pm` | The same lines with CRLF and a comma: `resets Sep 22, 8:40pm` | Run it; compare with the fixtures in `macos/Tests/CCUsageCoreTests/ProbeTests.swift` and `windows/crates/router-core/tests/probe_tests.rs` |
| Does setting `CLAUDE_CONFIG_DIR` to the default path differ from unsetting it? | Yes (logged out) | Not tested; the port keeps the macOS rule | Try both |

Write what you find in the port issue. Even "same as macOS" is a finding.

## What to replace

### Credential store — `KeychainStore` (macOS), `CredentialStore` (Windows)

The protocol is four operations: `read`, `write`, `exists`, `delete`, keyed by
the service name the adapter derives from the profile path.

If Claude Code on your platform stores the credential in a **file** inside the
profile, the implementation is a file write — but **write a fresh file** (a temp
file, then a rename), never a copy that keeps the source's timestamp: on Windows
a live session only re-reads the credential when its modification time changes,
so a timestamp-preserving copy swaps nothing. The invariants don't relax either:
the rotating-refresh-token problem is about two live copies, not about keychains.
Mirror-before-swap and one-account-one-place still apply.

If it uses a system secret store (`libsecret`, Windows Credential Manager),
mirror `SecurityCLIKeychain`: use the same tool Claude Code uses to write, so
you don't trip an authorization prompt; never put the secret on `argv`.

### Process liveness — `ProcessLiveness`

Two facts: does the pid exist, and did that process start when the registry
says? Linux: `/proc/<pid>/stat`, field 22 (`starttime`, in clock ticks since
boot) plus `/proc/stat` `btime`. Windows (done): `OpenProcess` +
`GetProcessTimes`. The tolerance and the "can't prove it, trust the pid"
fallback should carry over as they are.

### Sign-in — `LoginSession`

The app runs `claude auth login` in a pseudo-terminal, reads the output for the
authorization URL, and waits for `Login successful`. It runs in a pty on
purpose: under a plain pipe the CLI buffers its output and never prints the
link. Linux has `openpty`; Windows uses ConPTY (see `PLATFORM.md` for the
cursor-position request it waits on). The outcome check is on disk
(`AccountLoginService`) and is portable.

Strip proxy and credential-override variables from the environment before
launching (`ProviderEnv.direct`). That list is platform-independent.

### Shell integration — `ShellIntegration`

A shell function that shadows `claude`, asks `router is-group`, and runs
`router launch`. The zsh text becomes bash/fish/PowerShell text; the
`statusLine` entry in `settings.json` is the same JSON everywhere. Keep the
**append-in-bytes** behaviour when editing the user's profile file: reading it
as UTF-8 and rewriting it destroyed a `~/.zshrc` once. Expect the user's profile
to define `claude` already — a function or an alias — and chain it rather than
fight it.

The silent failure mode — a terminal opened before the integration — exists on
every platform. The sessions registry is the detector.

### Data directory — `RouterPaths`

`~/Library/Application Support/com.synqo.falcao-router/` on macOS and
`%LOCALAPPDATA%\com.synqo.falcao-router` on Windows — a fixed name, deliberately
not the app's bundle id (see the warning below). On Linux it would be
`$XDG_DATA_HOME/com.synqo.falcao-router` (or `~/.local/share/…`). Everything
under it — `config.json`, `accounts/`, `groups/`, `usage/`, `statusline.json`,
`shell.sh` — is relative.

**Warning:** on macOS the keychain service name is a hash of the profile path,
so moving the data directory orphans every credential. If your credential store
is path-keyed too, the same applies. Pick the location once.

### UI

`MenuBarExtra` becomes a StatusNotifierItem / AppIndicator on Linux, as it
became a tray icon on Windows. The panel is a popover of: groups → accounts →
two windows each, with the active one marked; a detail card; and a footer.
`LSUIElement` + "Show in Dock" becomes "start minimized to tray" or similar.
`SMAppService` becomes an autostart entry.

The one thing to carry over exactly: **every number shows its window, its
source and its age.** That's an invariant, not a style.

## What to keep

- `GroupModel`, `AccountModel`, `ConfigDir`, `RouterConfig` — the config
  format, as JSON. A port that reads and writes the same `config.json` can share
  a data directory with another platform's app on a synced machine (don't —
  credentials don't sync — but the format allows it).
- `GroupUsageSample`, `GroupUsageStore`, `GroupUsageReader` — the sample
  format and the decision.
- `StatusLineChoice` — `statusline.json`, the items the user keeps on the line.
- `RotationEngine` — the rules. Every line of it exists because its absence
  killed an account.
- `ClaudeUsageProbe.parse` and `resetDate` — the `/usage` parser, with its
  time formats (Windows adds the comma) and the nearest-year rule.
- `SessionRegistry.session(from:)` — the registry record decoder.
- `ProviderEnv` — the environment scrub.
- The tests. `EngineTests`, `StoreTests`, `ProbeTests`, `SessionRegistryTests`
  run against in-memory fakes and don't touch the platform; `router-core`'s
  suite does the same in Rust.

## What "done" looks like

- The port issue documents every verified fact from the table above.
- `router statusline`, `launch`, `is-group`, `rotate`, `measure` and `doctor`
  work, or their equivalents do.
- The invariants hold, and there's a test for each rule in `RotationEngine`.
- A person who has never seen the other apps can install it, create a group,
  sign two accounts in, run `claude <group>`, and watch it switch — from the
  README alone.
- No real account appears anywhere in the diff.
