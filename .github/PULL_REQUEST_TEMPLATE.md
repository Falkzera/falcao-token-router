<!--
Keep PRs to one concern. A PR that fixes a bug and reorganizes three files is
two PRs that are harder to review and harder to revert.

Title = a Conventional Commit line (feat:, fix:, docs:, refactor:, chore:, ci:).
It becomes the squash commit's subject on main.
-->

## What this changes

<!-- And, more usefully: why. What did the previous behaviour get wrong? -->

## What it deliberately doesn't fix

<!--
Optional, but it's the part reviewers most want to read. If you found a related
problem and left it alone on purpose, say so here so nobody re-derives it later.
-->

## Checks

- [ ] The suite of the platform you touched passes — `macos/Scripts/test.sh`, `windows/scripts/test.ps1`, or both
- [ ] `swift build -c release` passes
- [ ] New user-facing strings have keys in **both** `en.lproj` and `pt-BR.lproj`
- [ ] Logic changes in `CCUsageCore` come with tests
- [ ] No `@State` (use `@ViewState`), no SwiftUI import in `CCUsageCore`
- [ ] `agent.md` of every touched folder updated if a file, decision or gap changed
- [ ] No real account e-mail, employer name or measured usage anywhere in the diff

## Invariants

<!-- Delete this section if the change doesn't go near credentials, the keychain, rotation or measurement. Otherwise, keep it and add the `privacy` label. -->

- [ ] Adds **no** network call (the app has none)
- [ ] Adds no reader for `refreshToken`, and no OAuth refresh
- [ ] Goes through `RotationEngine.activate` (mirror before swap; one account, one place)
- [ ] Never probes the home of an active account (`probeConfigDir`)
- [ ] Keychain access stays on `/usr/bin/security`
- [ ] Every number shown still carries its window, source and age
