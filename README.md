<div align="center">

<img src="app-icon.svg" width="88" alt="Zap icon">

# Zap

**type, zap, done.** A private text expander for Windows.

Type a short trigger in any app and Zap turns it into your saved text: emails, addresses, replies, code.

[**Download for Windows**](https://github.com/meojaswi/Zap/releases/latest) · [Website](https://zap-rho-five.vercel.app) · [Report a bug](https://github.com/meojaswi/Zap/issues)

<img src="docs/demo.gif" width="800" alt="In a Windows text editor, typing `email expands to an email address and `today to the date as DD/MM/YYYY; Ctrl+Shift+Space opens Zap's search box, which pastes a saved reply; then Zap's window shows the time it has saved">

</div>

## Features

- **Inline expansion**: type a trigger like `` `addr `` in any app (browser, Slack, Word, your editor) and it's replaced instantly.
- **Search popup**: press `Ctrl+Shift+Space` anywhere to search snippets by name, trigger or text, then press `Enter` to paste.
- **Placeholders**: `{date}`, `{time}`, `{clipboard}` and more are filled in at the moment a snippet is inserted.
- **Sensitive snippets**: typed key by key so they never touch the clipboard, and hidden in the list and search.
- **Excluded apps**: switch off inline expansion in games, password managers, remote-desktop apps or anything else.
- **Usage stats**: see how often you use each snippet and roughly how much typing time Zap has saved you.
- **Stays out of the way**: lives in the tray, starts with Windows, and pauses with a double-click on the tray icon.
- **Import / export** snippets as JSON.
- **Automatic updates**: Zap tells you when a new version is out and installs it in one click.

## Using Zap

### Triggers

A trigger is the short text you type to insert a snippet. It needs at least 2 characters and no spaces, and one trigger can't contain another (otherwise the shorter one would always fire first).

Starting triggers with `` ` `` keeps them from firing by accident, but some websites block special characters like `` ` `` or `$` in their fields.

For those, set a **secondary trigger prefix** once in **Settings**, made of letters like `zz`. Every trigger that starts with a symbol then also works with the prefix in its place, so `` `email `` also works as `zzemail` and `` `today `` as `zztoday`. Triggers that already start with a letter work everywhere, so they stay as they are. You can change the prefix any time, or clear it to turn this off. You can also insert any snippet from the search popup.

### Starter snippets

New installs come with these, so you can try Zap straight away. Edit or delete them as you like.

| Trigger        | Inserts                  |
| -------------- | ------------------------ |
| `` `date ``      | today's date             |
| `` `now ``       | the date and time        |
| `` `time ``      | the current time         |
| `` `day ``       | the day of the week      |
| `` `clipboard `` | whatever you last copied |
| `` `uuid ``      | a fresh random ID        |

### Placeholders

Placeholders go inside a snippet's text; they aren't triggers on their own. For example, a snippet with the trigger `` `now `` and the text `{datetime}` expands to the current date and time.

| Placeholder   | Becomes                   |
| ------------- | ------------------------- |
| `{date}`      | `2026-11-01`              |
| `{time}`      | `14:05`                   |
| `{datetime}`  | `2026-11-01 14:05`        |
| `{day}`       | `Sunday`                  |
| `{clipboard}` | whatever you last copied  |
| `{uuid}`      | a fresh random ID         |

For other date and time formats, add format codes after a colon. Plain `{date}` and `{time}` keep the formats above.

| You write                          | Becomes                |
| ---------------------------------- | ---------------------- |
| `{date:DD/MM/YYYY}`                | `06/11/2026`           |
| `{date:D MMM YYYY}`                | `6 Nov 2026`           |
| `{date:dddd, D MMMM}`              | `Friday, 6 November`   |
| `{time:hh:mm A}`                   | `05:05 PM`             |
| `{time:HH:mm:ss}`                  | `17:05:09`             |
| `{datetime:DD/MM/YYYY hh:mm A}`    | `06/11/2026 05:05 PM`  |

| Code   | Means               | Code   | Means                 |
| ------ | ------------------- | ------ | --------------------- |
| `YYYY` | year, `2026`        | `HH`   | hour, 24-hour, `17`   |
| `YY`   | year, `26`          | `H`    | hour, 24-hour, no zero |
| `MMMM` | month, `November`   | `hh`   | hour, 12-hour, `05`   |
| `MMM`  | month, `Nov`        | `h`    | hour, 12-hour, `5`    |
| `MM`   | month, `11`         | `mm`   | minutes, `05`         |
| `M`    | month, no zero      | `ss`   | seconds, `09`         |
| `DD`   | day, `06`           | `A`    | `AM` / `PM`           |
| `D`    | day, `6`            | `a`    | `am` / `pm`           |
| `dddd` | weekday, `Friday`   |        |                       |
| `ddd`  | weekday, `Fri`      |        |                       |

Codes are case-sensitive: `MM` is the month and `mm` is minutes. Put any other words in square brackets, like `{date:[Week of] D MMM}`, so their letters aren't read as codes.

### Sensitive snippets

Tick **Sensitive** in the editor for private details like an ID number. Normal snippets are pasted through the clipboard for a moment; Sensitive ones are typed key by key instead, so no other program can read them from the clipboard. They're also marked **[locked]** in the list, and their text is hidden in the search popup and left out of search.

Typing takes longer than pasting, at about 50 characters a second, so keep your hands off the keyboard until it finishes. Every snippet is encrypted on disk either way, and exports include Sensitive snippets as plain text.

### Usage stats

Zap counts each time a snippet is inserted, whether from a trigger or the search popup:

- **Header:** your total time saved, like `⚡ 1h 9m saved`.
- **Editor:** under each snippet's text, like `Used 128 times · last used today · saved ~8m`.
- **Settings → Stats:** your totals, your top 5 snippets, your typing speed, an on/off switch and **Reset stats**.

Time saved is an estimate: the characters you didn't have to type (the snippet's length minus the trigger's), at your typing speed. The default is 40 words a minute, and a word counts as 5 characters. Text inserted by `{clipboard}` doesn't count, since you could have pasted it anyway.

### Excluded apps and remote sessions

Triggers don't fire in apps listed under **Settings → Excluded apps**, one program name per line, like `valorant.exe`.

Add remote-desktop apps there too: `teamviewer.exe`, `anydesk.exe` and `mstsc.exe` (Windows Remote Desktop). Your keyboard is still connected to your own PC during a remote session, so your Zap sees what you type and expands your snippets on the other computer, including Sensitive ones. Excluding these apps, or pausing Zap from the tray, stops that.

### Tray menu and shortcuts

| Tray icon         | Does                                                          |
| ----------------- | ------------------------------------------------------------- |
| Left click        | Opens Zap                                                     |
| Double left click | Pauses or resumes Zap                                         |
| Right click       | Menu: open, search snippets, pause, check for updates or quit |

While Zap is paused, its tray icon turns grey with a pause badge and triggers don't fire.

Closing the Zap window keeps Zap running in the tray, and it reopens on the Snippets tab. To stop it completely, use **Quit Zap** in the tray menu.

In the Zap window:

| Shortcut | Action          |
| -------- | --------------- |
| `Ctrl+N` | New snippet     |
| `Ctrl+S` | Save snippet    |
| `Ctrl+F` | Search the list |

### Updates

Zap checks GitHub for a new version when it starts and about once an hour. When one is out, you get a Windows notification and an **Install and restart** banner. You can also use **Check for updates** in the tray menu or **Settings → Updates → Check now**, and turn automatic checks off in Settings.

### Anonymous user count

So we can see how many people use Zap, it checks in once a day, at most, with two things:

- a **random ID**, made on your PC the first time. It isn't based on your hardware, name, email or snippets.
- the **version number**, like `0.1.14`.

Nothing else is sent: not your snippets, not what you type, not your stats. Our server doesn't even keep the ID; it adds it to approximate counters (about 1% error) and discards it.

It's on by default. Turn it off under **Settings → Anonymous user count**, and Zap deletes the ID straight away. Development builds never check in.

## Privacy

Inline expansion needs a keyboard hook, so Zap is built to keep as little as possible:

- Snippets are **encrypted on disk** with Windows DPAPI, so only your Windows account can read them.
- Pasted text is kept **out of Win+V clipboard history and cloud sync**, and your previous clipboard text is put back.
- Snippets marked **Sensitive** are typed key by key and never touch the clipboard.
- Keystrokes are **never logged or saved**. Only the last few characters are held in memory to spot triggers.
- **Usage stats stay on your PC.** Zap counts how often each snippet is used and the last date, to show the time it saves you. Nothing you type is recorded, stats are encrypted with your snippets and left out of exports, and you can turn them off or reset them in Settings.
- **No account. Only an anonymous user count.** Zap connects to GitHub to check for updates, and once a day to our server so we can count active users. That check-in sends only a random ID made on your PC and the version number: nothing about you, your PC or your snippets, and the server keeps a count, not the ID. You can turn either off in **Settings**; turning off the user count deletes the ID.
- **Signed updates.** Zap notifies you when a new version is out and refuses any update that isn't signed.

Don't store passwords in Zap. A password manager is the right place for those.

### If someone else gets your laptop

Encryption protects your snippets while your PC is **locked or switched off**: without your Windows sign-in, nobody can read them, even with the drive taken out. But anyone using your **unlocked** Windows account counts as you, so they can open Zap, see your snippets and use your triggers.

To keep details like ID numbers safe:

- Lock your PC whenever you step away: `Win + L`.
- Turn on a short screen timeout, and under **Settings → Accounts → Sign-in options** set Windows to ask you to sign in again **every time** you've been away.
- Turn on **Device encryption** or **BitLocker** under **Settings → Privacy & security**.
- Use a PIN that isn't easy to guess, and turn on **Find my device**.

## Install

1. Download `Zap_x.y.z_x64-setup.exe` from the [latest release](https://github.com/meojaswi/Zap/releases/latest).
2. Run it. No admin rights are needed; it installs just for your user.
3. Zap opens with a few starter snippets. Type `` `date `` in Notepad to try it.

After that, Zap keeps itself up to date, so you only need to download it once.

> **"Windows protected your PC"?** The installer isn't code-signed yet. Click **More info → Run anyway**.

Requires Windows 10 or 11 (64-bit).

## Build from source

You need [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 18+ and the WebView2 runtime (included with Windows 11).

```sh
npm install
npm run dev     # run the app with hot reload
npm run build   # build the installer → src-tauri/target/release/bundle/nsis/
```

`npm run build` signs the installer for the updater, so it needs `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` set. Without them, the installer is still built but the command ends with an error.

## Releasing

Change the version in `src-tauri/tauri.conf.json` and push to `main`. GitHub Actions builds, signs and publishes the release, and installed copies notify their users within about an hour.

### Counting users

The site's `/api/ping` endpoint receives the daily check-ins and stores only HyperLogLog counts in Neon Postgres. To see them, open:

```
https://zap-rho-five.vercel.app/api/stats?token=<STATS_TOKEN>
```

| Field                | Means                                                    |
| -------------------- | -------------------------------------------------------- |
| `activeToday`        | Installs that checked in today (UTC)                     |
| `activeLast7Days`    | Different installs seen in the last 7 days               |
| `activeLast30Days`   | Different installs seen in the last 30 days              |
| `installsEver`       | Every install that has ever checked in: the all-time total |
| `versionsLast7Days`  | Active installs per app version over the last 7 days     |
| `daily`              | Active installs for each of the last 30 days             |

Each install counts once, however often it checks in. A person on two PCs counts as two, people who turn the check-in off aren't counted, and counting started with 0.1.14, so earlier users only appear once they update. `STATS_TOKEN` is a secret environment variable on the Vercel project; without it the page returns 404.

## Project structure

```
├── src/            App UI: manager window and search popup (plain HTML/JS)
├── src-tauri/      Rust backend (Tauri 2)
│   └── src/
│       ├── hook.rs        keyboard hook for inline expansion
│       ├── expand.rs      placeholders + paste/type delivery
│       ├── input.rs       synthetic keystrokes for typing and pasting
│       ├── clipboard.rs   clipboard access, kept out of Win+V history
│       ├── store.rs       snippets and settings
│       ├── crypto.rs      DPAPI encryption
│       ├── updater.rs     signed update checks and installs
│       └── usage.rs       anonymous daily check-in
├── site/           Landing page (React + TypeScript + Tailwind), deployed to Vercel
│   └── api/        Vercel functions: /api/ping and /api/stats for the user count
└── docs/           README assets
```

The landing page is a separate project. Run it with `cd site && npm install && npm run dev`.
