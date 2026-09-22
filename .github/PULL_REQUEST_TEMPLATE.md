<!--
Keep PRs to one concern. A PR that fixes a bug and reorganizes three files is
two PRs that are harder to review and harder to revert.
-->

## What this changes

<!-- And, more usefully: why. What did the previous behaviour get wrong? -->

## What it deliberately doesn't fix

<!--
Optional, but it's the part reviewers most want to read. If you found a related
problem and left it alone on purpose, say so here so nobody re-derives it later.
-->

## Checks

- [ ] `./Scripts/test.sh` passes (core suite + string catalogs)
- [ ] New user-facing strings have keys in **both** `en.lproj` and `pt-BR.lproj`
- [ ] Logic changes in `CCUsageCore` come with tests
- [ ] No new SwiftUI import in `CCUsageCore`, and no `@State` (needs Xcode-only macros)

## Privacy invariants

<!-- Delete this section if the change doesn't go near credentials or the network. -->

- [ ] Doesn't add a reader for `refreshToken`
- [ ] Doesn't extract `accessToken` outside the live-fetch toggle
- [ ] Doesn't add a second network endpoint
- [ ] Doesn't increase how often the Keychain is read
