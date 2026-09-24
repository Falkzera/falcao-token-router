# Porting to Linux or Windows

This app is macOS-only today. The maintainer works on macOS and can't test
elsewhere — so a port is yours to own, and this document exists to make that
tractable: it lists what a port must replace, what it can keep, and what it
must verify before trusting.

Open a **port** issue first so the work is visible. Use a `port/<platform>`
branch. The review will be about the [invariants](../CONTRIBUTING.md#the-invariants),
which hold on every platform, not about the platform itself.

## Two ways to do it

**Reuse the core.** `CCUsageCore` is plain Foundation, and Swift runs on Linux
and Windows. The platform seams are small: a `KeychainStore` implementation, a
`ProcessLiveness` implementation, the sign-in driver, the shell integration
text, and the data directory. Replace those, keep the engine, tests and CLI,
and write a native UI on top.

**Rewrite.** Any language, reading and writing the same files. The contracts are
the file formats and the CLI behaviour, all described in `ARCHITECTURE.md`.
This is more work up front and loses the test suite, but it's a legitimate
choice if Swift on your platform isn't where you want to be.

Either way, the sensor is the smallest and most portable piece — a program that
reads JSON from stdin and writes one file. Start there: it proves the status
line delivers `rate_limits` on your platform, which everything else depends on.

## What to verify first

Do this before writing any code. Each is a fact about Claude Code on **your**
platform, and the macOS answer may not transfer.

| Question | macOS answer | How to check |
|---|---|---|
| Where does a profile keep its credential? | Keychain item `Claude Code-credentials[-<hash>]` | Sign in with `CLAUDE_CONFIG_DIR=/tmp/p claude auth login`, then look for a `.credentials.json` inside the profile, or a system secret store entry |
| Is the credential a file? | No | If yes, the "keychain store" for your port is a file copy — simpler than macOS |
| Does `.claude.json` live beside the default profile and inside a dedicated one? | Yes | `ls -la ~ ~/.claude /tmp/p` after signing in to each |
| Does the status line receive `rate_limits`? | Yes | Set `statusLine` to a script that dumps stdin to a file; send one message |
| Is `<profile>/sessions/<pid>.json` written? | Yes | Open a session, `ls <profile>/sessions/`. The record's `pidDomain` field names the platform |
| What does `claude --print "/usage"` print? | Three `Current …` lines | Run it; compare with the fixture in `macos/Tests/CCUsageCoreTests/ProbeTests.swift` |
| Does setting `CLAUDE_CONFIG_DIR` to the default path differ from unsetting it? | Yes (logged out) | Try both |

Write what you find in the port issue. Even "same as macOS" is a finding.

## What to replace

### Credential store — `KeychainStore`

The protocol is four operations: `read`, `write`, `exists`, `delete`, keyed by
the service name the adapter derives from the profile path.

If Claude Code on your platform stores the credential in a **file** inside the
profile, the implementation is a file copy and the service name is just the
profile path. That is *simpler* than macOS, but the invariants don't relax: the
rotating-refresh-token problem is about two live copies, not about keychains.
Mirror-before-swap and one-account-one-place still apply.

If it uses a system secret store (`libsecret`, Windows Credential Manager),
mirror `SecurityCLIKeychain`: use the same tool Claude Code uses to write, so
you don't trip an authorization prompt; never put the secret on `argv`.

### Process liveness — `ProcessLiveness`

Two facts: does the pid exist, and did that process start when the registry
says? Linux: `/proc/<pid>/stat`, field 22 (`starttime`, in clock ticks since
boot) plus `/proc/stat` `btime`. Windows: `OpenProcess` + `GetProcessTimes`.
The tolerance and the "can't prove it, trust the pid" fallback should carry
over as they are.

### Sign-in — `LoginSession`

The app runs `claude auth login` in a pseudo-terminal, reads the output for the
authorization URL, and waits for `Login successful`. It runs in a pty on
purpose: under a plain pipe the CLI buffers its output and never prints the
link. Linux has `openpty`; Windows has ConPTY. The outcome check is on disk
(`AccountLoginService`) and is portable.

Strip proxy and credential-override variables from the environment before
launching (`ProviderEnv.direct`). That list is platform-independent.

### Shell integration — `ShellIntegration`

A shell function that shadows `claude`, asks `router is-group`, and `exec`s
`router launch`. The zsh text becomes bash/fish/PowerShell text; the
`statusLine` entry in `settings.json` is the same JSON everywhere. Keep the
**append-in-bytes** behaviour when editing the user's profile file: reading it
as UTF-8 and rewriting it destroyed a `~/.zshrc` once.

The silent failure mode — a terminal opened before the integration — exists on
every platform. The sessions registry is the detector.

### Data directory — `RouterPaths`

`~/Library/Application Support/<bundle-id>/` becomes `$XDG_DATA_HOME` (or
`~/.local/share`) on Linux and `%LOCALAPPDATA%` on Windows. Everything under it
— `config.json`, `accounts/`, `groups/`, `usage/`, `shell.sh` — is relative.

**Warning:** on macOS the keychain service name is a hash of the profile path,
so moving the data directory orphans every credential. If your credential store
is path-keyed too, the same applies. Pick the location once.

### UI

`MenuBarExtra` becomes a StatusNotifierItem / AppIndicator on Linux or a tray
icon on Windows. The panel is a popover of: groups → accounts → two windows
each, with the active one marked; a detail card; and a footer. `LSUIElement` +
"Show in Dock" becomes "start minimized to tray" or similar. `SMAppService`
becomes an autostart entry.

The one thing to carry over exactly: **every number shows its window, its
source and its age.** That's an invariant, not a style.

## What to keep

- `GroupModel`, `AccountModel`, `ConfigDir`, `RouterConfig` — the config
  format, as JSON. A port that reads and writes the same `config.json` can share
  a data directory with the macOS app on a synced machine (don't — keychain
  items don't sync — but the format allows it).
- `GroupUsageSample`, `GroupUsageStore`, `GroupUsageReader` — the sample
  format and the decision.
- `RotationEngine` — the rules. Every line of it exists because its absence
  killed an account.
- `ClaudeUsageProbe.parse` and `resetDate` — the `/usage` parser, with its two
  time formats and the nearest-year rule.
- `SessionRegistry.session(from:)` — the registry record decoder.
- `ProviderEnv` — the environment scrub.
- The tests. `EngineTests`, `StoreTests`, `ProbeTests`, `SessionRegistryTests`
  run against in-memory fakes and don't touch the platform.

## What "done" looks like

- The port issue documents every verified fact from the table above.
- `router statusline`, `launch`, `is-group`, `rotate`, `measure` and `doctor`
  work, or their equivalents do.
- The invariants hold, and there's a test for each rule in `RotationEngine`.
- A person who has never seen the macOS app can install it, create a group,
  sign two accounts in, run `claude <group>`, and watch it switch — from the
  README alone.
- No real account appears anywhere in the diff.
