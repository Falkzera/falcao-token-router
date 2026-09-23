# Windows: verified facts

What the Windows port relies on, and how each fact was checked. Verified on
Windows 11 with the native install of Claude Code **2.1.280**, on 2026-09-22, in
isolated profiles (never the machine's own `%USERPROFILE%\.claude`). Paths, names
and numbers below are anonymized.

None of this is documented by Anthropic. Where a fact came from reading the
JavaScript embedded in `claude.exe`, it says so — treat those as "true for this
version" and re-check on upgrades.

## The table from `docs/PORTING.md`

| Question | Windows answer | How it was checked |
|---|---|---|
| Where does a profile keep its credential? | A file, `<profile>\.credentials.json` (default profile: `%USERPROFILE%\.claude\.credentials.json`). Nothing in Credential Manager. | `claude auth login` into empty profiles; official docs agree |
| Is the credential a file? | **Yes** — a JSON blob `{claudeAiOauth: {…}, mcpOAuth: {…}?}` | Only key names were inspected, never values |
| `.claude.json` beside the default profile, inside a dedicated one? | **Yes**, same asymmetry as macOS | `%USERPROFILE%\.claude.json` vs `<profile>\.claude.json` |
| Does the status line receive `rate_limits`? | **Yes**: `five_hour`/`seven_day` with `used_percentage` and `resets_at` (epoch s) — but **not** on the first render of a session | A logging status line in a dedicated profile |
| Is `<profile>\sessions\<pid>.json` written? | **Yes**. `procStart` is a **FILETIME** (100 ns since 1601 UTC, as a string) that matches the process start exactly; `pidDomain` is `win32:<host>` with the DNS host name in lowercase (e.g. `win32:exemplo-pc`) | Compared with `Process.StartTime` of the live process |
| What does `claude --print "/usage"` print? | Three `Current …` lines, CRLF, `·` (U+00B7), dates **with a comma**: `resets Sep 22, 8:40pm (America/Sao_Paulo)` / `resets Sep 23, 4am (…)` — macOS prints `Sep 22 at 8:40pm` | Captured and turned into a fixture (`probe_tests.rs`) |
| Logged-out profile? | Exit code **0** and no `Current` line at all — only the `--print` cost summary. "Not signed in" is decided by the absence of the lines, not the exit code | Probe against an empty profile |
| `CLAUDE_CONFIG_DIR` set to the default path vs unset? | An empty dedicated profile comes up logged out. The exact "unset vs explicitly `~\.claude`" pair was not tested (it would mean touching the real default profile); the port keeps the macOS rule: the default profile exports **no** variable | — |

## The hot swap

**Confirmed end to end.** A live session in a dedicated profile, served by account
A: `<profile>\.credentials.json` was replaced by B's (temp file + rename, so a new
mtime) and B's identity written to `<profile>\.claude.json`, without restarting the
session. On the next request carrying `rate_limits`, the status line reported B's
windows (the `resets_at` values changed to B's). Claude Code did not write A's
token or identity back.

Why it works (read in the JS of 2.1.280): before deciding whether to refresh, Claude
Code `stat`s `.credentials.json` and drops its cached credential when the **mtime**
changed. So every write of a credential must produce a new mtime — temp + rename of
freshly written bytes, never `CopyFile` (which keeps the source's mtime). Writing the
same bytes again is skipped on purpose: a new mtime for nothing makes every live
session re-read.

Claude Code itself writes the credential by staging + rename, with an in-place
fallback. The port only propagates a blob that parses as complete JSON with a
`claudeAiOauth` object (a structural check that skips every value), and re-reads
briefly when it finds a partial file.

`CLAUDE_SECURESTORAGE_CONFIG_DIR` is consulted **before** `CLAUDE_CONFIG_DIR` to find
the credential (JS of 2.1.280), so the port strips it — with the rest of the
credential/endpoint overrides the binary accepts — from every process it launches,
and `router doctor` reports it when set.

## The status line

- It runs through **Git Bash** when Git for Windows is installed, PowerShell
  otherwise. Claude Code looks for bash in this order: `CLAUDE_CODE_GIT_BASH_PATH`,
  `%ProgramFiles%\Git\bin\bash.exe`, `%ProgramFiles(x86)%\Git\bin\bash.exe`, then the
  `git.exe` on `PATH`.
- A path with **forward slashes and no quotes** works in both shells; quoted, it
  breaks in PowerShell. The port writes `C:/…/router.exe statusline`, falls back to
  the 8.3 short name when the path has a space, and only then to shell-specific
  quoting (`& '…'` for PowerShell).
- **stdin may never be closed.** Dozens of hung status-line processes were observed.
  The sensor parses the first complete JSON value, has a 250 ms deadline and always
  exits.
- The **first render has no `rate_limits`** (they arrive after the first API
  response). The macOS sensor writes an empty sample there, which erases the last
  reading; the port writes nothing without a window.
- `CLAUDE_CONFIG_DIR` reaches the status line process (it is not scrubbed by
  default). The port still embeds `--profile <dir>` in each dedicated group's
  command, used only when the variable is missing.
- A **project** `.claude\settings.json` overrides the group's status line — and when
  the working directory is the user's home, `~\.claude\settings.json` counts as
  project settings. `router doctor` warns about a competing project status line.
- Claude Code rewrites `<profile>\.claude.json` many times per session (always a new
  file), but keeps the identity the router wrote.

## PowerShell and Git Bash

- A literal `--` reaches a native executable, but a PowerShell **function** drops it
  from `$args` (5.1 and 7). `router launch` accepts both `<group> -- args` and
  `<group> args`.
- Windows PowerShell 5.1 reads a script without a BOM as ANSI, and saves profiles
  as UTF-16LE ("Unicode"). `shell.ps1` is UTF-8 **with** BOM; the line added to a
  profile is ASCII and is appended in the profile's own encoding and line ending.
- `$PROFILE` lives under the real Documents folder, which OneDrive may redirect
  (`SHGetKnownFolderPath`, not `%USERPROFILE%\Documents`).
- A user's profile may already define `function claude` (it did on the test machine).
  The integration saves it and calls it for `claude` without a group, instead of
  replacing it.
- Execution policy: the effective one must be computed **ignoring the Process
  scope** — the shell Claude Code runs commands in inherits `Bypass`. `Restricted`
  (the Windows PowerShell 5.1 default on client editions) keeps the profile from
  running, and `claude <group>` silently falls through to plain `claude`.
- Git Bash prints a red warning and creates a `~/.bash_profile` when it finds a
  `~/.bashrc` without one; the port creates the same file first.

## Sharing the profile

- **Directories** (`skills`, `commands`, `agents`, and `projects` with shared
  history) are **junctions** — no privilege needed. A transcript written through
  the group's `projects` junction lands in `~\.claude\projects`.
- **File** symlinks need Developer Mode (or admin). Without them, `CLAUDE.md` and
  `keybindings.json` are synced copies (newest wins, the overwritten side is backed
  up) and `history.jsonl` stays per group. **No hardlinks:** 2.1.280 prunes
  `history.jsonl` by rewriting it when it is a regular file (and skips links), so a
  hardlink would silently diverge.
- Deleting a folder that holds these junctions leaves their targets alone with
  `cmd /c rd /s /q`, and also with `Remove-Item -Recurse -Force` in PowerShell 7.6
  and in Windows PowerShell 5.1.26100 — checked on 2026-09-23 against a junction to
  a nested throwaway folder. Other builds of 5.1 weren't tried, so the README points
  to `rd`, which doesn't depend on the PowerShell version.

## Signing in

The app runs the official `claude auth login` in a **ConPTY** (through
`portable-pty` 0.9), in the account's own profile, and only reads the output to
find the link and to know when it ended; the outcome is then confirmed on disk
(identity in `.claude.json` **and** a credential). macOS does the same with a pty
(`docs/PORTING.md`: without a terminal the link doesn't come out in time).

What `claude auth login` prints (read in the JS of 2.1.280): `Opening browser to
sign in…`, then `If the browser didn't open, visit: <URL>` — the URL wrapped in an
OSC 8 hyperlink when stdout is a terminal — and `Paste code here if prompted > `.
It succeeds with `Login successful.` and **exits 0 by itself** (the "Press Enter to
continue" belongs to the interactive `/login`); it fails with `Login failed:
<reason>` on stderr and exit code 1. A pasted code must look like `code#state`;
anything else prints `Invalid code. …` and the login keeps waiting. `--email`
pre-fills the account, which the port passes when re-logging an existing account.

What the ConPTY does, recorded on Windows 11 (2026-09-23) with a stand-in `claude`
that prints the same text:

- `portable-pty` creates the ConPTY with `PSEUDOCONSOLE_INHERIT_CURSOR`, so the very
  first bytes are a cursor-position request, `ESC[6n`; with that flag the ConPTY
  waits for the terminal's answer. The port answers `ESC[1;1R`, as a terminal would.
- The ConPTY re-renders the output instead of passing the child's bytes through: a
  window-title OSC, `ESC[?9001h` (win32-input-mode), focus reporting, colours — and
  the OSC 8 hyperlink **re-emitted** as `ESC]8;id=<n>;<URL>ESC\`. The port strips
  VT/ANSI as a stream (a sequence may be cut between two reads) and takes the link
  from the hyperlink, or from the visible text only once it's complete, and only
  for `https://claude.com/` and `https://platform.claude.com/`. The pseudo-console
  is 2048 columns wide so the visible link doesn't wrap.
- Plain text followed by `\r` still arrives as a typed line, despite the
  win32-input-mode request.
- `portable-pty` builds the child's base environment from the process **and the
  registry** (user and system variables). The port clears it and passes only the
  filtered environment (no proxies or alternative credentials, none of a
  surrounding Claude Code session's variables, the account's `CLAUDE_CONFIG_DIR`).

## Packaging

An NSIS installer built by the Tauri CLI 2.11.5. Checked on 2026-09-23 in the
`installer.nsi` that the build generates (`target\release\nsis\x64\`):

- It installs **per user**, in `%LOCALAPPDATA%\FalcaoTokenRouter`, without
  administrator rights. The folder takes its name from `productName`, which is plain
  ASCII so that the path the status line cites gains no space or accent.
- `router.exe` ships as a **sidecar** (`bundle.externalBin`), and lands **beside the
  app's exe** under its name without the target triple
  (`File /a "/oname=router.exe"`). The app finds it there, next to its own
  executable, and on every start points the terminal integration at it — so a
  reinstall in another folder heals itself.
- The sidecar is declared in a config that only the installer build merges
  (`tauri build --config src-tauri/tauri.installer.conf.json`). The Tauri build
  script copies every `externalBin` into `target\<profile>\` on **every** cargo build
  of the app: declared in `tauri.conf.json`, it would break a clean build (the file
  doesn't exist yet) and overwrite the workspace's freshly built `router.exe` — the
  one the CLI tests run — with the last installer's copy.
- Without `mainBinaryName`, the installed exe keeps cargo's name
  (`falcao-token-router.exe`); the port sets it to `FalcaoTokenRouter`.
- The installer is in English and Brazilian Portuguese; with no language selector,
  NSIS picks the one that matches the Windows display language. WebView2 comes
  through the embedded bootstrapper, which downloads the runtime only when it is
  missing.
- The uninstaller removes the app's exe, `router.exe`, `uninstall.exe`, the
  shortcuts, the uninstall key and the `FalcaoTokenRouter` value under
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (the name the autostart
  plugin uses). It never touches `%LOCALAPPDATA%\com.synqo.falcao-router`, where the
  accounts' credentials are. Only with *Delete the application data* checked does it
  also remove `%APPDATA%` and `%LOCALAPPDATA%\com.synqo.falcao-token-router` (the
  app's settings and WebView2 cache) and `HKCU\Software\synqo\FalcaoTokenRouter`
  (the install location, kept for a reinstall).
- The installer isn't code-signed, so SmartScreen stops it once. A build made on the
  same machine carries no Mark of the Web and isn't stopped.

## Mapping from macOS

| Piece | macOS | Windows |
|---|---|---|
| Credential store | Keychain item via `/usr/bin/security` | `<profile>\.credentials.json`, opaque blob, temp + rename |
| Process liveness | `kill(pid,0)` + `sysctl` start time | `OpenProcess` + `GetProcessTimes` vs `procStart` FILETIME (same 300 s tolerance; "can't prove it, trust the pid") |
| `launch` | `execvp` | spawn inheriting the console, own Ctrl+C handler, wait, forward the exit code |
| Shell integration | zsh function in `~/.zshrc` | `shell.ps1` in both `$PROFILE`s, `shell.sh` in `~/.bashrc` (Git Bash) |
| Data directory | `~/Library/Application Support/com.synqo.falcao-router` | `%LOCALAPPDATA%\com.synqo.falcao-router` (Local, not Roaming) |
| Finding `claude` | three separate searches | one resolver: `ROUTER_CLAUDE_BIN`, `~\.local\bin\claude.exe`, `PATH`, `%APPDATA%\npm` (an npm `claude.cmd` shim is read and run as `node cli.js`, never through `cmd.exe`); Claude Desktop's copy and WindowsApps aliases are skipped |

## Deliberate differences

The port does not reproduce these macOS behaviours (each has a regression test):

- an empty sample written on the first status-line render;
- an unreadable `.claude.json` or `settings.json` replaced by `{}`;
- `exec` in the shell function, which closes an interactive shell when the session
  ends, and a silent fall-through when the `router` binary is gone;
- three different searches for the `claude` binary;
- the sensor ignoring `ROUTER_APP_SUPPORT`;
- `ProviderEnv` not stripping `CLAUDE_SECURESTORAGE_CONFIG_DIR`, `CLAUDE_CODE_OAUTH_TOKEN`
  and related overrides;
- no lock between the app and the CLI (the port uses a named mutex around every
  credential write);
- reordering accounts dropping one that the requested order forgot.

And these in the app (checked in the browser against the mocked backend, and in the
app itself inside a sandbox):

- "Installed ✓" shown even when writing the terminal integration failed;
- reordering accounts in the Groups window doing nothing (`onMove` outside a `List`);
- the threshold saved on every step of the slider, instead of when it's released;
- the "delete group" confirmation counting accounts that other groups keep;
- `lastError` written in Portuguese by the engine (the port returns facts with a
  code, and the text comes from the UI's catalogs);
- a login spinner that never stops when the account doesn't show up on disk (the
  port shows a named state with "Check again");
- a relogin that comes back as **another** account leaving that account's
  credential in this account's home, where "Use" would serve the other account
  under this one's name (the port removes it);
- the rotation pass running during a sign-in, where mirroring an active account
  (group → home) could overwrite the credential a relogin just wrote (the port
  waits until the sign-in ends).
