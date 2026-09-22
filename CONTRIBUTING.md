# Contributing

Thanks for looking. This is a small project with strong opinions, and most of
them are written down — so if something here seems arbitrary, it probably has a
reason worth arguing with. Arguing is welcome; that's what issues are for.

**Table of contents:** [Setting up](#setting-up) · [How the code is laid out](#how-the-code-is-laid-out) ·
[The workflow](#the-workflow) · [What will fail your PR](#what-will-fail-your-pr) ·
[The invariants](#the-invariants) · [Issues](#issues) · [Porting to other platforms](#porting-to-other-platforms) ·
[Language](#language) · [Security](#security) · [License](#license)

## Setting up

You need **macOS 26+** and **Swift 6.4+**. Command Line Tools is enough; Xcode is
not required, and the project is deliberately built so it never becomes required.

```bash
git clone https://github.com/Falkzera/falcao-token-router.git
cd falcao-token-router
./Scripts/test.sh                       # should end with "262 tests ... passed"
./Scripts/bundle.sh --native --install  # builds and copies to /Applications
```

An app you built yourself never carries the quarantine flag, so Gatekeeper stays
out of your way while developing.

### `swift test` does not work here, and that is not your setup

Command Line Tools ships swift-testing but doesn't wire it up: the macro plugin
sits outside the plugin path, and `Testing.framework` / `lib_TestingInterop.dylib`
sit outside the test bundle's rpath. `Scripts/test.sh` injects all three and
forwards arguments, so `./Scripts/test.sh --filter Probe` works normally.

### Never write `@State`

From the macOS 26 SDK, `@State` is a SwiftUI macro, and the `SwiftUIMacros`
plugin only ships with Xcode. Under Command Line Tools every use of it is a
compile error, and the whole app target stops building — that happened once, and
cost two weeks of commits that were never compiled.

Write **`@ViewState`** instead (`Sources/FalcaoTokenRouter/ViewState.swift`): a
typealias to `SwiftUICore.State`, the property wrapper the macro wraps, which *is*
in the SDK. Same type, same `$binding`, no plugin. CI builds without Xcode, so a
PR that reintroduces `@State` fails there too.

`@Observable` is unaffected — its plugin does ship in CLT.

## How the code is laid out

Three targets, and the split is load-bearing:

| Target | What it is | Imports SwiftUI? |
|---|---|---|
| `CCUsageCore` | The engine: groups, rotation, credential mirroring, the sensor and the probe, the meter. | **No.** All logic lives here, which is what makes it testable without a window. |
| `FalcaoTokenRouter` | The SwiftUI app: menu bar, panel, groups, settings — and **every user-facing string**. | Yes |
| `router` | The CLI the app bundles: `statusline` (the sensor), `launch`, `is-group`, `rotate`, `measure`, `doctor`. | No |

If you want to put a sentence in the core, that's the signal you're putting
presentation in the wrong place. `AlertPolicy` returns the fact — which window,
what percentage — never the wording. That separation is what lets the app be
localized without localizing the core.

**Every folder with code has an `agent.md`**: what the folder is for, what each
file does in one line, local conventions, dated decisions, and known gaps. Read
it before touching a folder. If your change adds a file, makes a decision or
resolves a gap, update it. That file is written for the next person — human or
LLM — and it's often the fastest way to understand *why* something is the way it
is. `docs/ARCHITECTURE.md` is the map of the whole thing.

## The workflow

GitHub Flow. One long-lived branch (`main`), protected: nothing lands on it
except through a pull request with green CI.

1. **Fork** the repo (or branch directly, if you have write access).
2. **Branch from an up-to-date `main`.** Name it by intent, in lowercase kebab-case:

   | Prefix | For |
   |---|---|
   | `feature/` | something the app doesn't do yet |
   | `fix/` | something it does wrong |
   | `docs/` | documentation only |
   | `refactor/` | no behaviour change |
   | `chore/` | build, CI, dependencies |
   | `port/` | a new platform (see [Porting](#porting-to-other-platforms)) |

   `feature/dock-icon`, `fix/zshrc-append`, `port/linux-tray`.

3. **Commit with [Conventional Commits](https://www.conventionalcommits.org/):**
   `feat:`, `fix:`, `docs:`, `refactor:`, `chore:`, `test:`, `ci:`. A scope in
   parentheses is welcome: `fix(sensor): ...`.

   The body explains **why**, including what the change deliberately does not
   fix. Read `git log` for the pattern — the bodies are long on purpose, because
   the reasoning is the part that's expensive to reconstruct later.

4. **Open a PR against `main`.** The template asks for the *why* and for what you
   left alone. Keep PRs to one concern: a PR that fixes a bug and reorganizes
   three files is two PRs that are harder to review and harder to revert.

5. **CI has to be green.** It runs the string check, the suite and a release
   build on `macos-26`. You can run the same thing locally with
   `./Scripts/test.sh && swift build -c release`.

6. **Squash merge.** Your commits become one on `main`, with the PR title as the
   subject — so make the title a good Conventional Commit line.

There is no `develop` branch, no release branch, and no direct push to `main`,
not even for the maintainer.

## What will fail your PR

### The string check

Every user-facing string is a key resolved against `Resources/en.lproj` and
`Resources/pt-BR.lproj`. Adding a raw literal to a view compiles, passes tests,
and ships a broken UI to whoever isn't reading your language — the compiler has
no way to know.

`Scripts/check-strings.sh` runs first in `Scripts/test.sh` and in CI. It fails
on a key missing from any catalog, an orphan translation, or a loose literal in a
view. `Text(verbatim:)` is the explicit way out for what genuinely isn't
translated (an account name, a number). Keys are namespaced: `panel.`, `settings.`,
`alerts.`, `format.`, `groups.`, `home.` — a new surface gets a new prefix, added
to the script. **English is the base**: it's what most people can read, so it's
the fallback.

### Missing tests

Logic changes in `CCUsageCore` come with tests. Most of this codebase was written
test-first and that's the expectation for the core — not ceremony, just that
behaviour worth having is behaviour worth pinning down. Every real bug caught in
use became a regression test that names the episode; that's a good habit to
continue.

UI-only changes are exempt; there's no view test harness, and adding one isn't a
prerequisite for fixing a label.

### Real accounts in the diff

Tests and examples use `conta1@exemplo.com`, `/Users/exemplo`, organization
`Acme`. **No real e-mail, employer, or measured usage of a real account** — in
code, comments, fixtures, screenshots or commit messages. This repository's
history was restarted once because of exactly that, and it will be enforced in
review without exception.

## The invariants

Some changes deserve more scrutiny than their diff size suggests. These are
guarantees the README makes to users, and the reasons behind the product's whole
design:

- **The app makes no network calls. None.** Not to Anthropic, not for telemetry,
  not to check for updates. Usage comes from the official client — the status
  line hands us `rate_limits`, and the probe asks `claude /usage`. A PR that adds
  *any* endpoint will be declined regardless of quality. If something needs the
  network, the answer is "ask the official binary to do it".
- **The app never refreshes OAuth, and `refreshToken` has no reader.** The
  guarantee is structural: `ClaudeCredentials` has no field to hold it. Don't
  add one. Rotation copies a keychain *blob* between items; it never decodes it
  to use a token.
- **One account, one place.** The engine refuses to activate an account already
  active in another group. Two live copies of a rotating refresh token kill one
  of them silently — that killed real accounts before this rule existed.
- **Mirror before you swap.** While an account is active, the *group's* keychain
  item is the live one. Before activating anyone else, the fresh token goes back
  to the leaving account's home. Never bypass `RotationEngine.activate`.
- **Never probe the home of an active account.** `probeConfigDir` decides where
  to measure. Probing the home would renew with a stale refresh token, rotate
  the chain and drop the user's live session into "Login expired".
- **Keychain access goes through `/usr/bin/security`**, the same binary Claude
  Code uses to write the items — that is what keeps macOS from prompting on every
  read. `PlanDetector` is the one exception, and it's listed as a known gap, not a
  pattern to copy.
- **A number on screen always says where it came from** — which window, which
  source (sensor or probe), how old. A proposal that simplifies the display by
  dropping provenance is the wrong trade for this app.

If a change touches any of these, say so in the PR and add the `privacy` label.
That's not a hurdle — it's the part a reviewer most wants to read.

## Issues

Pick the template that fits: **bug**, **feature**, or **port** (a new platform).
Labels `good first issue` and `help wanted` mark things a newcomer can pick up;
`needs-triage` means nobody has looked yet.

For bugs, the single most useful thing you can paste is the output of:

```bash
'/Applications/FalcaoTokenRouter.app/Contents/MacOS/router' doctor
```

It names the problem in most of the failure modes this app has, and it prints no
secrets — but **do redact account e-mails** before pasting; they're yours, and
they don't need to be public. Also say which group you were in, whether the
number came from the sensor or a probe (the panel says), and how you installed.

Feature requests are welcome. Describe the situation you're in rather than the
solution you have in mind: the best features here came from someone describing a
moment where the app left them guessing.

## Porting to other platforms

This is a macOS app today, and the maintainer works on macOS. **Ports are
welcome**, and the codebase was split so that one is feasible: everything
platform-specific sits behind a small number of seams, and the file formats the
app reads and writes are the same everywhere Claude Code runs.

`docs/PORTING.md` maps each macOS-specific piece to what a Linux or Windows port
needs to replace, and what it can keep. Open a **port** issue first so the work
is visible and nobody duplicates it; use a `port/` branch; and expect the review
to be about the invariants above, not about the platform — they hold everywhere.

Two honest caveats. The maintainer can't test on your platform, so a port lives
or dies on its own tests. And the credential mechanism was discovered by
observing Claude Code on macOS; on another platform, verify every assumption in
`docs/ARCHITECTURE.md` against your own install before building on it.

## Language

Code comments in this repo are in **Portuguese**, by choice — it's the
maintainer's language, and `CCUsageCore` is mostly comments explaining decisions.
Everything facing the outside world — README, `docs/`, UI strings, release notes,
this file — is in **English**, because it's what the most people can read.

Write your comments and PR description in whichever of the two you're
comfortable with. Don't translate existing comments as part of an unrelated
change. A Portuguese translation of the README would be a welcome first
contribution.

## Security

If you find something with security or privacy impact, please don't open a public
issue. Use GitHub's **Report a vulnerability** button under the Security tab.

## License

By contributing, you agree your contributions are licensed under the
[MIT License](LICENSE) that covers the project. The project is a fork of
[ClaudeTokenCounter](https://github.com/Ulpio/ClaudeTokenCounter) (MIT), whose
attribution is preserved in `LICENSE`.
