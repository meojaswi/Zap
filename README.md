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
- **Placeholders**: filled in at the moment a snippet is inserted.
- **Excluded apps**: switch off inline expansion in games, password managers or anything else.
- **Stays out of the way**: lives in the tray, starts with Windows, pauses with one click.
- **Import / export** snippets as JSON.

### Placeholders

Placeholders go inside a snippet's text; they aren't triggers on their own. For example, a snippet with the trigger `` `now `` and the text `{datetime}` expands to the current date and time. New installs include one example snippet for each placeholder.

| Placeholder   | Becomes                   |
| ------------- | ------------------------- |
| `{date}`      | `2026-11-01`              |
| `{time}`      | `14:05`                   |
| `{datetime}`  | `2026-11-01 14:05`        |
| `{day}`       | `Sunday`                  |
| `{clipboard}` | whatever you last copied  |
| `{uuid}`      | a fresh random ID         |

## Privacy

Inline expansion needs a keyboard hook, so Zap is built to keep as little as possible:

- Snippets are **encrypted on disk** with Windows DPAPI, so only your Windows account can read them.
- Pasted text is kept **out of Win+V clipboard history and cloud sync**, and your previous clipboard text is put back.
- Snippets marked **Sensitive** are typed key by key and never touch the clipboard.
- Keystrokes are **never logged or saved**. Only the last few characters are held in memory to spot triggers.
- **No account, no telemetry, no network connections.**

Don't store passwords in Zap. A password manager is the right place for those.

## Install

1. Download `Zap_x.y.z_x64-setup.exe` from the [latest release](https://github.com/meojaswi/Zap/releases/latest).
2. Run it. No admin rights are needed; it installs just for your user.
3. Zap opens with a few example snippets. Type `` `date `` in Notepad to try it.

> **"Windows protected your PC"?** The installer isn't code-signed yet. Click **More info → Run anyway**.

Requires Windows 10 or 11 (64-bit).

## Build from source

You need [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 18+ and the WebView2 runtime (included with Windows 11).

```sh
npm install
npm run dev     # run the app with hot reload
npm run build   # build the installer → src-tauri/target/release/bundle/nsis/
```

## Project structure

```
├── src/            App UI: manager window and search popup (plain HTML/JS)
├── src-tauri/      Rust backend (Tauri 2)
│   └── src/
│       ├── hook.rs        keyboard hook for inline expansion
│       ├── expand.rs      placeholders + paste/type delivery
│       ├── clipboard.rs   clipboard access, kept out of Win+V history
│       ├── store.rs       snippets and settings
│       └── crypto.rs      DPAPI encryption
├── site/           Landing page (React + TypeScript + Tailwind), deployed to Vercel
└── docs/           README assets
```

The landing page is a separate project. Run it with `cd site && npm install && npm run dev`.
