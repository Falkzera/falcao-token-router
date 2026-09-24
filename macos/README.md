# Falcão Token Router — macOS

The macOS app: a menu bar item that keeps several Claude Code accounts in groups
and swaps the active one when it hits the threshold, without ending your
session. What the product *is* and why it works is in the
[root README](../README.md); this file is how to install it, run it and build it
on a Mac.

**Requires macOS 26 (Tahoe) or newer.**

## Install

Grab `FalcaoTokenRouter-<version>.dmg` from the
[latest macOS release](../../../releases?q=macos-v), open it, and drag the app
onto *Applications*. The binary is universal — Apple Silicon and Intel.

> This repository tags each platform separately, so GitHub's *latest release*
> link is ambiguous: it may point at a Windows release. Follow the tag, or read
> the release title — it names the system.

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
cd falcao-token-router/macos
./Scripts/bundle.sh --native --install   # builds and copies to /Applications
```

An app you assembled yourself never carries the quarantine flag.

## Where things live

Everything the app writes is under
`~/Library/Application Support/com.synqo.falcao-router/`: `config.json` (groups
and accounts), `accounts/<uuid>/` (each account's home profile),
`groups/<uuid>/` (the profile a group's sessions run in), `usage/<email>.json`
(the sensor's samples) and `shell.sh` (the terminal function).

The credential itself is **not** there — it is a keychain item,
`Claude Code-credentials` for the default profile and
`Claude Code-credentials-<sha256(path)[:8]>` for any other, written by Claude
Code and read through `/usr/bin/security`.

> That hash is why the data folder is named `com.synqo.falcao-router` and not
> after the current product name. Renaming the base directory changes every
> profile path, changes every hash, and puts every credential out of reach at
> once — with no message that explains it.

## Development

```bash
./Scripts/test.sh          # core suite + string catalog check
./Scripts/check-strings.sh # keys vs. catalogs, and loose literals in views
./Scripts/icon.sh          # draws the .icns, installer art, banner, social card
./Scripts/bundle.sh        # assembles dist/FalcaoTokenRouter.app
./Scripts/dmg.sh           # builds dist/FalcaoTokenRouter-<version>.dmg
./Scripts/release.sh       # checks, tags macos-vX.Y.Z and pushes; CI builds the universal DMG
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

The universal build needs `xcbuild`, which comes only with the full Xcode, so it
does not run on a Command Line Tools machine — `bundle.sh --native` builds for
the host arch instead. The release workflow does the universal build in CI.

### Layout

| Target | What it is | Imports SwiftUI? |
|---|---|---|
| `CCUsageCore` | The engine: groups, rotation, credential mirroring, the sensor's store, the meter. | **No** — which is what makes every rule testable without a window. |
| `FalcaoTokenRouter` | The app: menu bar, panel, groups, settings — and every user-facing string. | Yes |
| `router` | The CLI the app bundles: `statusline` (the sensor), `launch`, `is-group`, `rotate`, `measure`, `doctor`. | No |

`Resources/{en,pt-BR}.lproj/` hold the string catalogs, checked by
`check-strings.sh`. Every folder with code has an `agent.md` — read it before
touching the folder.

## Cutting a release

1. Bump `VERSION`, write the section in [`CHANGELOG.md`](CHANGELOG.md), merge through a PR.
2. On a clean `main`: `./Scripts/release.sh` — it checks everything, tags `macos-vX.Y.Z` and pushes.
3. CI (`macos-26`, with Xcode) runs the suite from scratch, builds the universal app, the DMG and the zip, and opens the release as a **draft**.
4. Review the notes and publish: `gh release edit macos-vX.Y.Z --draft=false`.

The tag prefix matters: a bare `v*` tag triggers nothing any more.
