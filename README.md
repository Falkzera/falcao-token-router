![Falcão Token Router](docs/art/banner.png)

# Falcão Token Router

A macOS menu bar app that keeps several Claude Code accounts in **groups** and
switches the active one for you when it runs out — without ending your session.

```
◐ 81%  equipe-2   ← which account is serving you, and how much of it is spent
```

## What it does

You create groups — `trabalho`, `pessoal`, `faculdade` — and sign each account
in through Anthropic's own login flow, inside the app. You set the order and the
threshold. From then on:

```
claude trabalho   → the freest account in the "trabalho" group; swaps at the threshold
claude pessoal    → same, for the "pessoal" group
claude            → straight through to the binary, in ~/.claude
```

The swap happens **inside a live session**. Claude Code re-reads its keychain
item on the next request, so the account changes under a running conversation —
no restart, no `--resume`, no lost context.

## The constraint that designs everything

Anthropic's terms reserve the OAuth token to the official client. So:

- **The app makes no network calls of its own.** Not one. Usage is measured by a
  passive sensor: Claude Code's status line hands us the `rate_limits` block it
  already received from the API, and we read it from stdin.
- **The app never authenticates.** Sign-in runs the official `claude auth login`
  binary in a pty, in an isolated profile. The app watches the output for the
  link and watches the disk for the outcome. It never sees a password or a token.
- **The app never refreshes OAuth.** It copies a secret between keychain items;
  it never mints one.

The price of that is honest and visible: **an idle account shows `pronta`, not a
number.** There is no sample until that account serves a message. The app says so
rather than inventing a percentage.

## How the swap works

Each account has a **home** — `accounts/<uuid>`, its own config profile, its own
keychain item, written by Claude Code at login. Each group has a **profile**,
where its sessions run.

Activating an account in a group copies the secret from its home into the group's
keychain item and writes the identity into the group's `.claude.json`.

Two rules are what separate this from a shell script that gets it wrong:

- **Mirror before you swap.** While an account is active, it is the *group's*
  item that Claude Code refreshes, and the refresh token rotates on every
  renewal. Before activating anyone else, the fresh token is copied back to the
  leaving account's home. The home is always the truth when the account is idle.
- **One account, one place.** The same account active in two groups would be two
  copies of a rotating refresh token — which kills one of them silently. The
  engine refuses it.

## What it measures

The sensor writes one sample per account, keyed by e-mail. The rotation compares
the **larger** of the 5-hour and 7-day windows against the group's threshold.

A window whose reset has already passed is discarded rather than kept — otherwise
an old sample would leave an account looking permanently full.

Every number on screen carries its provenance: which window it came from, and how
old the sample is. Past an hour it fades; past twelve hours it gets an explicit
mark, because on a shared account an optimistic stale number is the one that
sends you into an account that is already spent.

### The probe, for what the sensor cannot see

Two things never reach the status line's `rate_limits`: the **per-model limit**
(the Fable ceiling that has actually locked accounts here) and any number at all
for an **idle account**, which has never served a message.

So there is a second, deliberate measurement — **Measure accounts** in a group,
or `router measure [group]`:

```
$ router measure trabalho
  equipe-1: 5h 2%   7d 3%    Fable 0%
  equipe-2: 5h 26%  7d 38%   Fable 0%
  equipe-3: 5h 10%  7d 70%   Fable 0%
```

It asks the official binary (`claude --print /usage`) — the same thing that
happens when you type `/usage` yourself. Still no network call of our own, still
no token read. It costs a Node cold start per account, so it is a button and a
command, never a loop; the passive sensor remains the thing that runs every
minute.

Per-model numbers carry their own timestamp, separate from the sensor's, because
the two age at different rates. An account active in a group is always probed
through the **group's** profile, never through its home — probing the home of an
active account would renew with the stale refresh token, rotate the chain, and
drop the live session into "Login expired".

## Install

### Download

Grab `FalcaoTokenRouter-<version>.dmg` from the
[latest release](../../releases/latest), open it, and drag the app onto
*Applications*. The binary is universal — Apple Silicon and Intel.

The app is **ad-hoc signed, not notarized** (that needs a paid Apple Developer
account, which is on the roadmap). macOS will refuse to open it the first time.
Clear the quarantine flag once, after copying it to *Applications*:

```bash
xattr -dr com.apple.quarantine /Applications/FalcaoTokenRouter.app
```

Or open it, let macOS block it, then **System Settings → Privacy & Security →
Open Anyway**. If macOS says the app is *damaged*, that is the quarantine flag,
not a bad download — the command above fixes it.

Then open the window (the app shows in the menu bar; **Settings → System → Show
in Dock** makes it a regular app), create a group, add accounts, and click
**Activate** under *Terminal integration*.

> **Open a new terminal afterwards.** The integration is a shell function that
> shadows the binary. In a terminal opened before the install, `claude trabalho`
> is just an argument to `claude` and your session silently opens in `~/.claude`,
> on the wrong account. `source ~/.zshrc` fixes an already-open terminal;
> `router doctor` tells you what's wrong.

### From source

No Xcode needed — Command Line Tools with Swift 6.4+ is enough.

```bash
git clone https://github.com/Falkzera/falcao-token-router.git
cd falcao-token-router
./Scripts/bundle.sh --native --install   # builds and copies to /Applications
```

An app you assembled yourself never carries the quarantine flag. Requires
**macOS 26+** and Swift 6.4+ — Command Line Tools is enough, Xcode is not needed.

## Languages

English and Brazilian Portuguese, following your system language. Any other
locale falls back to English.

## Privacy

The app runs entirely on your machine. **No server, no telemetry, no analytics,
and no network calls at all.**

**Local reads:** each profile's `.claude.json` (for the account identity Claude
Code wrote there), the status-line samples this app itself writes under
`Application Support`, and `~/.claude/projects/**/*.jsonl` (read-only, for the
token and cost meter).

**Keychain:** the `Claude Code-credentials` items, through `/usr/bin/security` —
the same binary Claude Code uses to write them, which is what keeps macOS from
prompting on every read. The app copies the blob between profiles. It never
decodes it to use a token, and `ClaudeCredentials` has no field for a refresh
token, so no code path can reach one.

## Development

```bash
./Scripts/test.sh          # core suite (245 tests) + string catalog check
./Scripts/check-strings.sh # keys vs. catalogs, and loose literals in views
./Scripts/icon.sh          # draws the .icns, installer art, banner, social card
./Scripts/bundle.sh        # assembles dist/FalcaoTokenRouter.app
./Scripts/dmg.sh           # builds dist/FalcaoTokenRouter-<version>.dmg
./Scripts/release.sh       # checks, tags and pushes; CI builds the universal DMG
```

**Plain `swift test` does not work** in this toolchain: Command Line Tools ships
swift-testing but doesn't wire it up — the macro plugin sits outside the plugin
path, and `Testing.framework` / `lib_TestingInterop.dylib` sit outside the test
bundle's rpath. `Scripts/test.sh` injects all three and forwards arguments, so
`./Scripts/test.sh --filter PricingTable` works normally.

For a related reason the app does not write `@State` directly: from the macOS 26
SDK it is a SwiftUI macro, and the `SwiftUIMacros` plugin ships only with Xcode —
under Command Line Tools every use of it is a compile error. `ViewState` (see
`Sources/FalcaoTokenRouter/ViewState.swift`) is a typealias to
`SwiftUICore.State`, the property wrapper the macro wraps, which *is* in the SDK.

Code comments are in Portuguese, by choice — it is the maintainers' language, and
`CCUsageCore` is mostly comments explaining decisions that were discovered by
observation and are expensive to rediscover.

### Architecture

The full map is [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md). In short:
three targets, and `CCUsageCore` imports no SwiftUI — all logic is testable
without instantiating a window.

| Target | What it is |
|---|---|
| `CCUsageCore` | The engine: groups, rotation, credential mirroring, the sensor's store, the meter. No UI. |
| `FalcaoTokenRouter` | The SwiftUI app: menu bar, panel, groups, settings. |
| `router` | The CLI the app bundles: `statusline` (the sensor), `launch <grupo>`, `is-group`, `rotate`. |

Everything provider-specific sits behind `ProviderAdapter` — the keychain item
name derived from the config directory, the `.claude.json` beside or inside it,
the launch command. `RotationEngine` talks only to the protocol, so a second
provider is a new adapter, not a new engine.

`AlertPolicy` is pure and takes no clock: the same sequence of snapshots produces
the same alerts, which is what makes rearming testable at all. The `Alert` type
carries the fact — which window, what percentage — never the sentence. Wording
lives in the app target with every other user-facing string.

## Platforms

macOS 26+ today, because that's what the maintainer runs and can test.

**Ports are welcome.** Everything platform-specific sits behind a few small
seams — the credential store, process liveness, the sign-in terminal, the shell
hook, the tray UI — and the files the app reads and writes are the ones Claude
Code writes the same way everywhere. [`docs/PORTING.md`](docs/PORTING.md) maps
each piece to what a Linux or Windows port needs to replace, what it can keep,
and what it must verify first. Open a [port issue](../../issues/new?template=port.yml)
to start one.

## Contributing

Issues and pull requests are welcome, and the project is set up for it:

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — building without Xcode, the branch and
  PR workflow, the two checks that fail a PR, and the invariants a change near
  credentials has to preserve.
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — how the swap, the sensor, the
  probe and the sessions registry actually work, and what's macOS-specific.
- Every folder has an `agent.md`: what it's for, what each file does, the
  decisions taken there and why. Read it before touching the folder.
- Issue templates for [bugs](../../issues/new?template=bug_report.yml),
  [features](../../issues/new?template=feature_request.yml) and
  [ports](../../issues/new?template=port.yml). `good first issue` and
  `help wanted` mark what a newcomer can pick up.
- CI runs the string check, the suite and a release build on every PR.

Bug reports: paste the output of `router doctor` (e-mails redacted). It names
the problem in most of the failure modes this app has.

Two things the project won't trade away: **no network calls of its own**, and
**every number says where it came from**.

## Lineage

Forked from [ClaudeTokenCounter](https://github.com/Ulpio/ClaudeTokenCounter) by
Ulpio (MIT), which was the meter this grew out of. The rotation engine, the
groups, the passive sensor and the CLI are new.

## License

MIT — see [LICENSE](LICENSE).
