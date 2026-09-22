# Contributing

Thanks for looking. This is a small project with strong opinions, and most of
them are written down — so if something here seems arbitrary, it probably has a
reason worth arguing with.

## Getting set up

You need **macOS 26+** and **Swift 6.4+** from Command Line Tools. Xcode is not
required and the project is deliberately built so it never becomes required.

```bash
git clone https://github.com/Falkzera/falcao-token-router.git
cd falcao-token-router
./Scripts/test.sh              # should end with "245 tests ... passed"
./Scripts/bundle.sh --install  # builds and copies to /Applications
```

An app you built yourself never carries the quarantine flag, so Gatekeeper stays
out of your way while developing.

### `swift test` does not work here, and that is not your setup

Command Line Tools ships swift-testing but doesn't wire it up: the macro plugin
sits outside the plugin path, and `Testing.framework` / `lib_TestingInterop.dylib`
sit outside the test bundle's rpath. `Scripts/test.sh` injects all three and
forwards arguments, so `./Scripts/test.sh --filter PricingTable` works normally.

For a related reason the code never writes `@State` directly: from the macOS 26
SDK it is a SwiftUI macro, and the `SwiftUIMacros` plugin only ships with Xcode —
under Command Line Tools every use of it is a compile error, and the whole app
target stops building. Write **`@ViewState`** instead: a typealias to
`SwiftUICore.State`, the property wrapper the macro wraps, which *is* in the SDK.
Same type, same `$binding`, no plugin. A PR that reintroduces `@State` will fail
to build for everyone without Xcode.

Observed redraw (`@Observable`) is unaffected — its plugin does ship in CLT.

## Where code goes

Three targets, and the split is load-bearing:

- **`CCUsageCore`** — does not import SwiftUI. All logic lives here, which is
  what makes it testable without instantiating a window. Pure functions where
  possible: `UsageSourcePolicy` picks between origins, `AlertPolicy` takes no
  clock, `GaugeGeometry` builds the ring path.
- **The UI target** — SwiftUI, and **every user-facing string**.

If you find yourself wanting to put a sentence in the core, that's the signal
you're putting presentation in the wrong place. `AlertPolicy` returns the fact —
which window, what percentage — never the wording. That separation is what lets
the app be localized without localizing the core.

## Two checks that will fail your PR

### Strings

Every user-facing string is a key resolved against `Resources/en.lproj` and
`Resources/pt-BR.lproj`. Adding a raw literal to a view compiles, passes tests,
and ships a broken UI to whoever isn't reading your language — the compiler has
no way to know.

So `Scripts/check-strings.sh` runs as part of `Scripts/test.sh` and fails if a
key is missing from any catalog, if a catalog carries an orphan translation, or
if a view holds a loose literal. **English is the base**: `CFBundleDevelopmentRegion`
is `en`, because the fallback should be what the most people can read.

### Tests

Logic changes come with tests. The suite was written first for most of this
codebase and that's the expectation for contributions to the core — not
ceremony, just that behaviour worth having is behaviour worth pinning down.

UI-only changes are exempt; there's no view test harness and adding one isn't a
prerequisite for fixing a label.

## The privacy invariants

The app reads Claude Code's Keychain item, so some changes deserve more scrutiny
than their diff size suggests. These are guarantees the README makes to users:

- **`refreshToken` has no reader anywhere in the app.** The guarantee is
  structural, not a promise: `ClaudeCredentials` has no field to hold it. Don't
  add one. The app never refreshes OAuth, because rewriting Claude Code's
  Keychain item could invalidate its session.
- **`accessToken` is extracted only behind the live-fetch toggle**, through a
  single point in the code. `PlanDetector` keeps its own credential source
  precisely so it can pass `readsAccessToken: false` — sharing the live source
  there would extract the token with fetching off.
- **One network call exists**: `GET https://api.anthropic.com/api/oauth/usage`,
  every 5 minutes, live only. A PR that adds a second endpoint — telemetry,
  analytics, update checks — will be declined regardless of quality.
- **Keychain reads are cached.** Each read can raise a macOS authorization
  dialog, so reading in a loop is a user-facing bug, not an efficiency one.
  Failed reads are cached too: retrying a denied authorization recreates exactly
  the problem the cache exists to solve.

If a change touches any of these, say so in the PR description. That's not a
hurdle — it's the part a reviewer most wants to read.

## Commits and pull requests

Commits use a Conventional Commits prefix (`feat:`, `fix:`, `docs:`, `refactor:`)
and a body that explains **why**, including what the change deliberately does not
fix. Look at `git log` for the pattern — the bodies are long on purpose, because
the reasoning is the part that's expensive to reconstruct later.

Before opening a PR:

```bash
./Scripts/test.sh    # core suite + string catalogs
```

Keep PRs to one concern. A PR that fixes a bug and reorganizes three files is two
PRs that are harder to review and harder to revert.

### About language

Code comments in this repo are in Portuguese, by choice — it's the maintainer's
language, and `CCUsageCore` is mostly comments explaining decisions. Everything
facing the outside world (README, UI strings, release notes) is in English.

Write your comments and PR description in whichever of the two you're
comfortable with. Don't translate existing comments as part of an unrelated
change.

## Issues

Bug reports are more useful than most projects' because this app can tell you
where its number came from. **Include the provenance line the panel shows** —
`live`, `Claude Code cache · 20m ago`, `stale cache · 18h old`, or
`estimated from your history`. A wrong percentage means something very different
depending on which one is on screen, and it's the first thing anyone debugging
will ask for.

Also include:

- Your macOS version and whether you're on Apple Silicon or Intel
- The app version (Settings shows it) and whether you installed from the DMG or
  built from source
- Whether live fetching is on

For anything involving Keychain prompts, mention whether you clicked **Allow** or
**Always Allow**, and whether you'd updated the app just before. An ad-hoc signed
build gets a new code signature hash on every build, so every install is a new
app to the Keychain and every "Always Allow" dies with the previous binary. That
one is known, and only a stable certificate fixes it.

Feature requests are welcome. The one thing the project won't trade away is that
a number on screen always says where it came from — a proposal that makes the
display simpler by dropping provenance is the wrong trade for this app.

## Security

If you find something with security or privacy impact, please don't open a public
issue. Use GitHub's **Report a vulnerability** button under the Security tab, or
email the address on the maintainer's GitHub profile.

## License

By contributing, you agree your contributions are licensed under the
[MIT License](LICENSE) that covers the project.
