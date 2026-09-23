# Falcão Token Router for Windows

A notification-area app that keeps several Claude Code accounts in **groups** and
switches the active one for you when it runs out — without ending your session.

This is the Windows port of the [macOS app](../README.md): the same idea, the same
rules and the same files, rewritten in Rust with a Tauri tray app. It covers the
router — groups, the automatic switch, the sensor, the probe and the `router`
command. The macOS app's token and cost meter is not part of it.

```
claude work       → a session on the "work" group's accounts; switches at the threshold
claude personal   → same, for the "personal" group
claude            → plain Claude Code, as before
```

## Before you start

- **Windows 11, x64.** Windows 10 version 1809 or later has everything the app
  uses, but it hasn't been tried there.
- **Claude Code**, installed: `claude --version` has to answer in a new terminal.
  The [official setup](https://code.claude.com/docs/en/setup) installs it with
  `irm https://claude.ai/install.ps1 | iex` in PowerShell; WinGet and npm installs
  work too. You don't have to sign in to it first — the app signs each account in,
  through the official `claude` itself.
- **Two or more Claude accounts** that can use Claude Code. The router is about
  taking turns between them.
- *Optional:* **Git for Windows**. When it's installed, Claude Code runs the status
  line through Git Bash, and `claude <group>` works in Git Bash as well.

## Install

There is no Windows release yet. Until there is one, take the installer from CI or
[build it yourself](#building-it-yourself).

**From CI.** Open the latest successful run of the
[*Windows* workflow](../../../actions/workflows/windows.yml) and download the
**FalcaoTokenRouter-setup** artifact at the bottom of the run's page (GitHub asks
you to sign in for that). Unzip it and run `FalcaoTokenRouter_<version>_x64-setup.exe`.

The installer is **not code-signed**, so Windows SmartScreen stops it the first
time: click **More info → Run anyway**. It installs for your user only, in
`%LOCALAPPDATA%\FalcaoTokenRouter`, without administrator rights. If WebView2 is
missing (Windows 11 always has it), the installer downloads and installs it.

On its first run the app opens its window. After that it lives in the
**notification area** — and Windows 11 hides every new icon behind the **^** on
the taskbar. Drag the ring from there onto the taskbar, or turn it on in
**Settings → Personalization → Taskbar → Other system tray icons**. If you'd
rather have a taskbar button, turn on the app's **Settings → Show in taskbar**: the
window opens with the app, and you can pin its button.

## Two accounts that take turns

A walkthrough from zero: one group, two accounts, and a switch you can watch.

### 1. Create a group

In the app's **Groups** tab, click **Create your first group**, name it `work` and
click **Create**. The name becomes the terminal command: `claude work`.

Leave **Use as the default group** off. A default group takes over plain `claude`
and your `~\.claude` profile; any other group gets a profile of its own and leaves
the rest of your setup alone.

### 2. Sign in two accounts

Click **Add account**. The app runs the official `claude auth login`, and your
browser opens Anthropic's sign-in page. Sign in with the first account; the app
says **Account added** once the login has landed on disk. If the browser didn't
open, the app shows the link — and if the page gives you a code to paste, the app
has a field for it.

Before adding the second account, **sign out at claude.ai** (or finish that sign-in
in a private window). Otherwise the browser signs the first account in again; the
app notices, says so, and offers a sign-out link and **Try again**.

Each account gets its own profile in the router's folder. Your password never
passes through the app, and neither does a token.

### 3. Turn on the terminal integration

Under **Terminal integration**, click **Activate**. That adds one line to your
PowerShell profiles and, if you have Git Bash, to `~/.bashrc`. Each shell then has a
row that says whether it loads the integration.

If a PowerShell row says the **execution policy** stops your profile from running,
click **Allow (RemoteSigned for your user)** — or run
`Set-ExecutionPolicy -Scope CurrentUser RemoteSigned` yourself. Without it,
`claude work` would quietly open plain `claude`, on the wrong account.

> **Open a new terminal afterwards.** The integration is a `claude` function your
> profile loads when a terminal starts. In a terminal opened before, `claude work`
> is just `claude` with an argument, and the session opens in `~\.claude`.

### 4. Start a session in the group

```powershell
claude work
```

The first line says which account serves the session — `→ work: conta1` — and
Claude Code starts in the group's profile. Whatever follows the group name goes to
Claude Code as usual: `claude work --resume`, `claude work -p "hi"`.

The **status line** at the bottom shows the group, the model and its effort level,
the git branch (or the folder), the context window, both usage windows with the time
they reset, the session's cost and the e-mail of the account serving you:

```
● work │ Opus 5.5 high │ main │ █████░░░░░ 51% 511k/1000k │ 5h █░░░░ 29% ↻ 14:05  7d ██░░░ 33% ↻ Mon (28) 9:00 │ $24.77 │ conta1@exemplo.com
```

The usage windows appear once the session has had its first answer. That line is
also the **sensor**: it hands the router the usage that Claude Code itself received —
so in a group, it takes the place of any status line you configured yourself (yours
keeps working in your other profiles). To trim the line, or to see your own status
line in group sessions, see [Status line](#status-line).

### 5. Watch the switch

With the session still open, click **Use** on the second account in the app. Then
send another message in the same session: the second account serves it, and the
e-mail at the end of the status line changes to it. No restart, no `--resume`.

The automatic switch is the same move. When the active account passes the group's
**Switch at** threshold, the router activates the first account in your order that
is below it — an account never measured counts as fresh. That is decided when you
start `claude work` and, with **Auto-switch** on, every 3 minutes while the app runs
— live sessions included. The tray ring and its tooltip follow the active account.

### 6. Check the setup

```powershell
& "$env:LOCALAPPDATA\FalcaoTokenRouter\router.exe" doctor
```

`router doctor` checks what fails **silently** — the profile lines, the execution
policy, each group's status line actually running through your shell, your status
line choice (and your own command, if you use one), the active accounts, live
sessions and the `claude` binary — and names what's wrong. Like the
macOS `router`, it prints in Portuguese: `ok` marks a check that passed, `!!` one
that didn't, and the last line is `tudo certo.` (all good) or `há problemas acima.`
(problems above).

## Using the app

- **The tray icon** is a ring filled with the active account's usage — green, then
  yellow from 66%, red from 90%. Its tooltip has one line per group: account,
  window, percentage, where the number came from and how old it is.
- **Left click** opens the accounts table next to the icon; click an account for
  its details. **Right click** has Groups, Settings and Quit.
- **Groups** has what the walkthrough used and, per group, the rotation order (drag,
  or ↑/↓), **Log in again…** for an account whose login expired, **Remove account**,
  **Measure accounts**, rename, default and delete.
- **Settings**: the **Status line** (below), **Open at login** and **Show in
  taskbar**.

Resize or maximize the window as you like; it reopens at its last size. Closing it
leaves the app running in the notification area; **Quit** ends it. The app follows your Windows language — English or Portuguese.

### Status line

**Settings → Status line** picks what group sessions show below the prompt. The
router measures usage whatever you pick, and a change applies at the next update of
the sessions already open.

- **Use the app's status line** (the default) is the full line shown in step 4.
  Untick what you don't want to see — group, model, effort, branch or folder,
  context, the 5-hour window, the 7-day window, reset times, cost, account e-mail —
  and the preview, drawn by the same code the sessions use, follows. With every item
  unticked, the line stays empty.
- Turn it off to run **your own command** instead — the status line you use in your
  other profiles, for example. After measuring, the router runs it with the same
  JSON Claude Code sends and the way Claude Code runs a status line: through Git
  Bash, or PowerShell when there's no Git Bash. **Test** runs it with a sample
  session. If your command fails, prints nothing or takes longer than 5 seconds, the
  session shows the app's line instead; whatever the command leaves running in the
  background ends with it.

The choice is kept in `%LOCALAPPDATA%\com.synqo.falcao-router\statusline.json`. If
that file is missing or unreadable, sessions show the full line.

## What the numbers mean

Every number carries its window, its source and its age: the tooltip reads
`work: conta1 · 7d 41% (sensor, 3m)`.

- The **sensor** reads the `rate_limits` of the requests an account actually
  served. An account that hasn't served a message yet shows **ready**, not a number.
- **Measure accounts** (or `router measure work`) asks the official `claude` for
  `/usage`, account by account. It sees idle accounts and the per-model limit, which
  never reaches the status line. It takes a few seconds per account, so it's a
  button, never a loop.

The switch compares the **larger** of the 5-hour and 7-day windows with the
threshold.

## Known issues

- **SmartScreen stops the installer**, because it isn't code-signed: **More info →
  Run anyway**.
- **The icon is hidden behind the ^** on the taskbar — Windows 11 does that to every
  new icon. Drag it out, or use **Settings → Show in taskbar** in the app.
- **`claude work` opens plain `claude`.** The terminal was opened before the
  integration (open a new one — or run `. $PROFILE` in PowerShell,
  `source ~/.bashrc` in Git Bash), or the execution policy blocks your profile (see
  step 3). `router doctor` says which.
- **Your own `claude` function** in your profile keeps working: the integration
  chains it, so plain `claude`, without a group, still goes through yours.
- **Developer Mode is off** (the Windows default): `CLAUDE.md` and
  `keybindings.json` are copied into each group on every `claude <group>` (the
  newest wins), and the ↑ prompt history is kept per group. With Developer Mode on,
  they become links shared by every group; the app has a button to its settings.
- **A project `.claude\settings.json` with its own `statusLine`** wins over the
  group's, and the group isn't measured in that project. `router doctor` warns
  about it.
- **The `router` command speaks Portuguese**, like the macOS one.

## Where things live

| What | Where |
|---|---|
| The app and `router.exe` | `%LOCALAPPDATA%\FalcaoTokenRouter` |
| Groups, accounts, their profiles, the samples | `%LOCALAPPDATA%\com.synqo.falcao-router` — the same layout as the macOS app's `Application Support` folder |
| The status line choice | `statusline.json` in that same folder (Windows only) |
| The app's own settings, and its window's cache | `%APPDATA%\com.synqo.falcao-token-router`, and the same name under `%LOCALAPPDATA%` |
| The terminal integration | one line in each `$PROFILE` and in `~/.bashrc`, pointing at `shell.ps1` / `shell.sh` in the router's folder |

## Updating

Run the newer installer over the old one. It closes the app if it's running (it asks
first) and replaces the program; your groups, accounts and settings stay. Sessions
you opened with `claude <group>` keep running: the installer moves the `router.exe`
they're using out of the way, and your next `claude <group>` runs the new one. Start
the app again afterwards — the installer's last page offers to.

## Uninstall

1. *Optional:* in the app, remove the accounts you added. That deletes their logins
   from this computer.
2. **Settings → Apps → Installed apps → FalcaoTokenRouter → Uninstall.** This
   removes the program. Your groups and accounts stay in
   `%LOCALAPPDATA%\com.synqo.falcao-router`, in case you reinstall. The
   uninstaller's **Delete the application data** box removes only the app's own
   settings and cache. A `claude <group>` session that is still open keeps running;
   the `router.exe` it uses goes to your `%TEMP%` folder.
3. Remove the integration: in each PowerShell, `notepad $PROFILE` and delete the
   line that mentions `com.synqo.falcao-router`; do the same in `~/.bashrc`. Until
   you do, `claude <group>` prints a red warning that `router.exe` is missing and
   runs plain `claude`.
4. To delete the groups and accounts as well:

   ```powershell
   cmd /c rd /s /q "%LOCALAPPDATA%\com.synqo.falcao-router"
   ```

   The groups' profiles hold links to folders in your `~\.claude` (`skills`,
   `projects`…). `rd` removes the links and leaves what they point to alone.

## Privacy

The same rules as the macOS app: **no server, no telemetry, and no network calls of
its own.** Signing in is the official `claude auth login`, measuring on request is
the official `claude /usage`, and the rest of the usage comes from the status line
that Claude Code runs.

On Windows, a Claude Code credential is a file, `<profile>\.credentials.json`. The
router copies it between an account's profile and a group's as an opaque blob: it
never decodes it, and no part of it reads a token.

## How it works on Windows

A live Claude Code session re-reads its credential file when the file's
modification time changes. A switch is therefore a fresh file written into the
group's profile (a temp file, then a rename), plus the account's identity in that
profile's `.claude.json`. Everything the port relies on, and how each fact was
checked, is in [`docs/PLATFORM.md`](docs/PLATFORM.md).

## Building it yourself

You need:

- **Rust**, through [rustup](https://rustup.rs): the stable MSVC toolchain, with
  the Visual Studio Build Tools and their *Desktop development with C++* workload;
- **Node.js** 22.12 or newer, with npm (the CI uses 24).

```powershell
git clone https://github.com/Falkzera/falcao-token-router.git
cd falcao-token-router\windows
.\scripts\build.ps1
```

`build.ps1` builds `router.exe`, the front end and the app in release, then the
installer, checks what came out and prints where it is (under
`target\release\bundle\nsis\`). The first build downloads NSIS and the WebView2
bootstrapper into `%LOCALAPPDATA%\tauri`. If PowerShell refuses to run scripts, use
`powershell -ExecutionPolicy Bypass -File .\scripts\build.ps1`.

## Development

```powershell
.\scripts\test.ps1   # fmt, clippy -D warnings, the tests, svelte-check, the string catalogs
cd app; npm run dev  # the window in a browser, against a mocked backend
```

| Folder | What it is |
|---|---|
| `crates/router-core` | The engine: groups, rotation, the credential file, the sensor's store, sessions, the terminal integration, the probe. No UI, no network. |
| `crates/router-cli` | `router.exe`: `statusline`, `launch`, `is-group`, `rotate`, `measure`, `doctor`. |
| `crates/gauge-mark` | The ring, drawn for the tray and for the app icon. |
| `crates/fake-claude` | A stand-in `claude` for the integration tests. Never shipped. |
| `app` | The Tauri v2 app (`src-tauri`) and its Svelte 5 front end (`src`). |

Code comments are in Portuguese, as in the rest of the repository, and every folder
has an `agent.md` saying what it's for and which decisions were taken there.
