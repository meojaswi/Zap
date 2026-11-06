<div align="center">

<img src="app-icon.svg" width="88" alt="Zap icon">

# Zap

**type, zap, done.** A private text expander for Windows.

Type a short trigger in any app and Zap turns it into your saved text: emails, addresses, replies, code.

[**Download for Windows**](https://github.com/meojaswi/Zap/releases/latest) · [Website](https://zap-rho-five.vercel.app) · [Report a bug](https://github.com/meojaswi/Zap/issues)

<img src="docs/demo.gif" width="800" alt="Typing `pemail expands to an email address, `date expands to today's date, and Ctrl+Shift+Space opens a search box that pastes a saved reply">

</div>

## Features

- **Inline expansion**: type a trigger like `` `pemail `` in any app (browser, Slack, Word, your editor) and it's replaced instantly.
- **Search popup**: press `Ctrl+Shift+Space` anywhere to search snippets by name, trigger or text, then press `Enter` to paste.
- **Placeholders**: `{date}`, `{time}`, `{clipboard}` and more are filled in at the moment a snippet is inserted.
- **Sensitive snippets**: typed key by key so they never touch the clipboard, and hidden in the list and search.
- **Excluded apps**: switch off inline expansion in games, password managers or anything else.
- **Stays out of the way**: lives in the tray, starts with Windows, pauses with one click.
- **Import / export** snippets as JSON.
- **Automatic updates**: Zap tells you when a new version is out and installs it in one click.

## Using Zap

### Triggers

A trigger is the short text you type to insert a snippet. It needs at least 2 characters and no spaces, and one trigger can't contain another (otherwise the shorter one would always fire first).

Starting triggers with `` ` `` keeps them from firing by accident, but some websites block special characters like `` ` `` or `$` in their fields. For those, use a letter-only trigger such as `zzemail`, or insert the snippet from the search popup instead.

### Starter snippets

New installs come with these, so you can try Zap straight away. Edit or delete them as you like.

| Trigger        | Inserts                  |
| -------------- | ------------------------ |
| `` `pemail ``    | an example email address |
| `` `date ``      | today's date             |
| `` `now ``       | the date and time        |
| `` `time ``      | the current time         |
| `` `day ``       | the day of the week      |
| `` `clipboard `` | whatever you last copied |
| `` `uuid ``      | a fresh random ID        |
| `` `iferr ``     | a Go error check         |

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

### Sensitive snippets

Tick **Sensitive** in the editor for private details like an ID number. Normal snippets are pasted through the clipboard for a moment; Sensitive ones are typed key by key instead, so no other program can read them from the clipboard. They're also marked **[locked]** in the list, and their text is hidden in the search popup and left out of search.

Typing takes longer than pasting, at about 50 characters a second, so keep your hands off the keyboard until it finishes. Every snippet is encrypted on disk either way, and exports include Sensitive snippets as plain text.

### Tray menu and shortcuts

Right-click the tray icon to open Zap, search snippets, pause Zap, check for updates or quit. In the Zap window:

| Shortcut | Action          |
| -------- | --------------- |
| `Ctrl+N` | New snippet     |
| `Ctrl+S` | Save snippet    |
| `Ctrl+F` | Search the list |

### Updates

Zap checks GitHub for a new version when it starts and about once an hour. When one is out, you get a Windows notification and an **Install and restart** banner. You can also use **Check for updates** in the tray menu or **Settings → Updates → Check now**, and turn automatic checks off in Settings.

## Privacy

Inline expansion needs a keyboard hook, so Zap is built to keep as little as possible:

- Snippets are **encrypted on disk** with Windows DPAPI, so only your Windows account can read them.
- Pasted text is kept **out of Win+V clipboard history and cloud sync**, and your previous clipboard text is put back.
- Snippets marked **Sensitive** are typed key by key and never touch the clipboard.
- Keystrokes are **never logged or saved**. Only the last few characters are held in memory to spot triggers.
- **No account, no telemetry.** Zap only connects to GitHub to check for updates, and you can turn that off in Settings. No snippets or usage data are sent.
- **Signed updates.** Zap notifies you when a new version is out and refuses any update that isn't signed.

Don't store passwords in Zap. A password manager is the right place for those.

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
│       └── updater.rs     signed update checks and installs
├── site/           Landing page (React + TypeScript + Tailwind), deployed to Vercel
└── docs/           README assets
```

The landing page is a separate project. Run it with `cd site && npm install && npm run dev`.
