const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (id) => document.getElementById(id);
const DEFAULT_HOTKEY = "Ctrl+Shift+Space";

let state = { snippets: [], settings: {}, autostart: false, paused: false, hotkeyError: null };
let selectedId = null; // "" while creating a new snippet
let toastTimer;

function toast(message, isError = false) {
  const el = $("toast");
  el.textContent = message;
  el.classList.toggle("error", isError);
  el.classList.add("show");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.remove("show"), isError ? 4500 : 2200);
}

function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML.replace(/"/g, "&quot;");
}

async function load() {
  state = await invoke("get_data");
  render();
}

function render() {
  renderStatus();
  renderList();
  renderSettings();
  renderStats();
  renderUsage();
  renderSecondary();
}

// ---------- Stats ----------

/** Seconds of typing saved: a word is 5 characters, at the user's words per minute. */
function secondsSaved(chars) {
  return (chars * 60) / (5 * (state.settings.typingWpm || 40));
}

function formatDuration(seconds) {
  if (seconds < 60) return `${Math.max(1, Math.round(seconds))}s`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
}

function formatLastUsed(date) {
  if (!date) return "";
  const days = Math.round((new Date(new Date().toDateString()) - new Date(date + "T00:00")) / 86400000);
  if (days <= 0) return "today";
  if (days === 1) return "yesterday";
  if (days < 30) return `${days} days ago`;
  return new Date(date + "T00:00").toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });
}

function totals() {
  return state.snippets.reduce((t, s) => ({ uses: t.uses + (s.uses || 0), chars: t.chars + (s.charsSaved || 0) }), { uses: 0, chars: 0 });
}

function renderStats() {
  const { uses, chars } = totals();
  const on = state.settings.keepStats;
  $("keep-stats").checked = on;
  const saved = $("saved-total");
  saved.hidden = !on || chars === 0;
  saved.innerHTML = `<span class="bolt">⚡</span> ${formatDuration(secondsSaved(chars))} saved`;

  $("stats-totals").innerHTML = !on
    ? "Stats are off, so nothing is being counted."
    : uses
    ? `<strong>${uses.toLocaleString()}</strong> expansions · <strong>${chars.toLocaleString()}</strong> characters · <strong>~${formatDuration(secondsSaved(chars))}</strong> saved`
    : "Nothing yet. Use a trigger and your stats show up here.";
  const top = state.snippets.filter((s) => s.uses > 0).sort((a, b) => b.uses - a.uses).slice(0, 5);
  $("stats-top").innerHTML = top
    .map(
      (s) => `<li><span class="item-trigger">${escapeHtml(s.trigger)}</span><span class="stats-name">${escapeHtml(s.name)}</span>
        <span class="stats-num">×${s.uses.toLocaleString()} · ~${formatDuration(secondsSaved(s.charsSaved || 0))}</span></li>`
    )
    .join("");
  if (document.activeElement !== $("wpm")) $("wpm").value = state.settings.typingWpm;
}

/** "Used 128 times · last used today · saved ~9 min" under the snippet's text in the editor. */
function renderUsage() {
  const usage = $("f-usage");
  const snippet = state.snippets.find((s) => s.id === selectedId);
  usage.hidden = !snippet || !state.settings.keepStats;
  if (!snippet) return;
  usage.textContent = snippet.uses
    ? `Used ${snippet.uses.toLocaleString()} ${snippet.uses === 1 ? "time" : "times"} · last used ${formatLastUsed(snippet.lastUsed)} · saved ~${formatDuration(secondsSaved(snippet.charsSaved || 0))}`
    : "Not used yet";
}

let restarting = false;

function renderStatus() {
  $("status").classList.toggle("paused", state.paused);
  $("status-text").textContent = state.paused ? "Paused" : "Active";
  $("welcome-hotkey").textContent = state.settings.hotkey;
  const banner = $("hotkey-banner");
  banner.hidden = !state.hotkeyError;
  banner.textContent = state.hotkeyError
    ? `${state.hotkeyError}. Another app may be using it, so pick a different one in Settings.`
    : "";
  // Once it's downloaded, the header button replaces the "available" banner.
  $("restart-update").hidden = !state.updateReady;
  if (state.updateReady && !restarting) $("restart-update").title = `Zap ${state.updateReady} is downloaded. Click to restart with it.`;
  $("update-banner").hidden = !state.updateVersion || Boolean(state.updateReady);
  $("update-text").textContent = `Zap ${state.updateVersion} is available.`;
}

// Collapsed group names, remembered per viewer. Storage can be unavailable, so it's optional.
const COLLAPSED_KEY = "zap.collapsedGroups";
let collapsed = new Set();
try {
  collapsed = new Set(JSON.parse(localStorage.getItem(COLLAPSED_KEY) || "[]"));
} catch {}

function toggleGroup(name) {
  collapsed.has(name) ? collapsed.delete(name) : collapsed.add(name);
  try {
    localStorage.setItem(COLLAPSED_KEY, JSON.stringify([...collapsed]));
  } catch {}
  renderList();
}

function groupNames() {
  return [...new Set(state.snippets.map((s) => (s.group || "").trim()).filter(Boolean))].sort((a, b) => a.localeCompare(b));
}

function itemHtml(s) {
  return `
      <button class="item ${s.id === selectedId ? "active" : ""}" data-id="${s.id}" title="${escapeHtml(s.name)}">
        <span class="item-trigger">${escapeHtml(s.trigger)}</span>
        <span class="item-name">${escapeHtml(s.name.toLowerCase())}${s.sensitive ? ' <span class="lock" title="Sensitive">[locked]</span>' : ""}</span>
      </button>`;
}

function renderList() {
  const query = $("search").value.trim().toLowerCase();
  const items = state.snippets
    .filter((s) => {
      if (!query) return true;
      const haystack = [s.name, s.trigger, s.group || "", s.sensitive ? "" : s.content].join(" ").toLowerCase();
      return haystack.includes(query);
    })
    .sort((a, b) => a.name.localeCompare(b.name));

  $("group-list").innerHTML = groupNames().map((g) => `<option value="${escapeHtml(g)}"></option>`).join("");

  const list = $("list");
  if (!items.length) {
    list.innerHTML = `<div class="empty">${state.snippets.length ? "No matches" : "No snippets yet"}</div>`;
    return;
  }
  // Without any groups, keep the plain list.
  if (!groupNames().length) {
    list.innerHTML = items.map(itemHtml).join("");
    return;
  }
  const sections = [...groupNames(), ""]
    .map((name) => ({ name, members: items.filter((s) => (s.group || "").trim() === name) }))
    .filter((section) => section.members.length);
  list.innerHTML = sections
    .map(({ name, members }) => {
      // While searching, show every match even in collapsed groups.
      const open = query || !collapsed.has(name);
      return `
      <button class="group-head" data-group="${escapeHtml(name)}" aria-expanded="${open}">
        <span class="group-arrow">${open ? "▾" : "▸"}</span>${escapeHtml(name || "Ungrouped")}<span class="group-count">${members.length}</span>
      </button>${open ? members.map(itemHtml).join("") : ""}`;
    })
    .join("");
}

function select(id) {
  selectedId = id;
  const snippet = state.snippets.find((s) => s.id === id);
  const creating = id === "";
  $("welcome").hidden = true;
  $("form").hidden = false;
  $("f-name").value = snippet?.name ?? "";
  $("f-trigger").value = snippet?.trigger ?? "`";
  $("f-content").value = snippet?.content ?? "";
  $("f-sensitive").checked = snippet?.sensitive ?? false;
  $("f-rich-editor").innerHTML = snippet?.html ? sanitizeHtml(snippet.html) : "";
  setRich(Boolean(snippet?.html));
  $("f-group").value = snippet?.group ?? "";
  $("delete-btn").hidden = creating;
  renderUsage();
  renderSecondary();
  updatePreview();
  renderList();
  (creating ? $("f-name") : isRich() ? $("f-rich-editor") : $("f-content")).focus();
}

function closeEditor() {
  selectedId = null;
  $("form").hidden = true;
  $("welcome").hidden = false;
  renderList();
  renderUsage();
}

// ---------- Formatting ----------

// Formatting a snippet can carry; everything else is unwrapped to its text, and links keep only
// web or mail addresses.
const RICH_TAGS = new Set(["B", "STRONG", "I", "EM", "U", "A", "BR", "P", "DIV", "UL", "OL", "LI"]);
const DROPPED_TAGS = new Set(["SCRIPT", "STYLE", "IFRAME", "OBJECT", "EMBED", "TEMPLATE", "HEAD", "TITLE", "META", "LINK"]);

function sanitizeHtml(html) {
  const template = document.createElement("template");
  template.innerHTML = html;
  const clean = (node) => {
    for (const child of [...node.childNodes]) {
      if (child.nodeType === Node.TEXT_NODE) continue;
      if (child.nodeType !== Node.ELEMENT_NODE || DROPPED_TAGS.has(child.tagName)) {
        child.remove();
        continue;
      }
      clean(child);
      if (!RICH_TAGS.has(child.tagName)) {
        child.replaceWith(...child.childNodes);
        continue;
      }
      const href = child.tagName === "A" ? (child.getAttribute("href") || "").trim() : "";
      for (const attr of [...child.attributes]) child.removeAttribute(attr.name);
      if (/^(https?:|mailto:)/i.test(href)) child.setAttribute("href", href);
    }
  };
  clean(template.content);
  return template.innerHTML;
}

function plainToHtml(text) {
  return escapeHtml(text).replace(/\n/g, "<br>");
}

function isRich() {
  return $("f-rich").checked;
}

/** The snippet's plain text: what's typed into apps without formatting, and what search uses. */
function currentContent() {
  return isRich() ? $("f-rich-editor").innerText.replace(/\n$/, "") : $("f-content").value;
}

function setRich(on) {
  $("f-rich").checked = on;
  $("f-content").hidden = on;
  $("f-rich-editor").hidden = !on;
  $("f-rich-toolbar").hidden = !on;
  // Formatted snippets are pasted, so they can't be Sensitive (typed) as well.
  if (on) $("f-sensitive").checked = false;
  $("f-sensitive").disabled = on;
  $("f-rich").disabled = !on && $("f-sensitive").checked;
}

let previewTimer;
function updatePreview() {
  clearTimeout(previewTimer);
  previewTimer = setTimeout(async () => {
    const content = currentContent();
    $("preview-field").hidden = !/\{(date|time|datetime|day|clipboard|uuid|input)(:[^}]*)?\}/.test(content);
    if (!$("preview-field").hidden) $("preview").textContent = await invoke("preview", { content });
  }, 150);
}

function insertPlaceholder(text) {
  if (isRich()) {
    $("f-rich-editor").focus();
    document.execCommand("insertText", false, text);
    updatePreview();
    return;
  }
  const area = $("f-content");
  const start = area.selectionStart;
  area.setRangeText(text, start, area.selectionEnd, "end");
  area.focus();
  updatePreview();
}

async function save(event) {
  event.preventDefault();
  try {
    const saved = await invoke("save_snippet", {
      snippet: {
        id: selectedId || "",
        name: $("f-name").value,
        trigger: $("f-trigger").value,
        content: currentContent(),
        html: isRich() ? sanitizeHtml($("f-rich-editor").innerHTML) : null,
        sensitive: $("f-sensitive").checked,
        group: $("f-group").value,
      },
    });
    await load();
    select(saved.id);
    toast("Saved ⚡");
  } catch (e) {
    toast(e, true);
  }
}

async function remove() {
  const snippet = state.snippets.find((s) => s.id === selectedId);
  if (!snippet || !confirm(`Delete "${snippet.name}"?`)) return;
  await invoke("delete_snippet", { id: snippet.id });
  closeEditor();
  await load();
  toast("Deleted");
}

// ---------- Secondary trigger ----------

/** Mirrors store::secondary_trigger: the prefix replaces a leading symbol; letter-led triggers keep theirs. */
function secondaryTrigger(trigger, prefix = state.settings.secondaryPrefix) {
  if (!prefix || !trigger || /^[\p{L}\p{N}]/u.test(trigger)) return null;
  return prefix + [...trigger].slice(1).join("");
}

function renderSecondary() {
  const hint = $("f-secondary");
  const alt = secondaryTrigger($("f-trigger").value.trim());
  hint.hidden = !alt;
  if (alt) hint.innerHTML = `Also works as <code>${escapeHtml(alt)}</code>`;

  if (document.activeElement !== $("secondary-prefix")) $("secondary-prefix").value = state.settings.secondaryPrefix || "";
  const example = state.snippets.map((s) => s.trigger).find((t) => secondaryTrigger(t)) || "`email";
  const preview = secondaryTrigger(example, $("secondary-prefix").value.trim());
  $("secondary-example").innerHTML = preview
    ? `<code>${escapeHtml(example)}</code> also works as <code>${escapeHtml(preview)}</code>`
    : "Off. Triggers only work as you wrote them.";
}

// ---------- Settings ----------

function renderSettings() {
  if (!$("hotkey").classList.contains("recording")) $("hotkey").value = state.settings.hotkey;
  $("inline").checked = state.settings.inlineEnabled;
  $("autostart").checked = state.autostart;
  $("check-updates").checked = state.settings.checkUpdates;
  $("auto-update").checked = state.settings.autoUpdate;
  $("auto-update").disabled = !state.settings.checkUpdates;
  $("usage-ping").checked = state.settings.usagePing;
  $("app-version").textContent = state.appVersion;
  if (document.activeElement !== $("excluded")) $("excluded").value = state.settings.excludedApps.join("\n");
}

async function updateSettings(patch, successMessage) {
  try {
    await invoke("update_settings", { settings: { ...state.settings, ...patch } });
    await load();
    if (successMessage) toast(successMessage);
  } catch (e) {
    toast(e, true);
    await load();
  }
}

const MODIFIER_CODES = ["ControlLeft", "ControlRight", "ShiftLeft", "ShiftRight", "AltLeft", "AltRight", "MetaLeft", "MetaRight"];
// Keys that are fine as a hotkey on their own because nothing types with them.
const SOLO_KEYS = /^(F(1[3-9]|2[0-4])|Pause|ScrollLock|Insert)$/;

function readableKey(code) {
  return code.replace(/^Key/, "").replace(/^Digit/, "");
}

function recordHotkey(event) {
  event.preventDefault();
  if (event.code === "Escape") {
    stopRecording();
    return;
  }
  if (MODIFIER_CODES.includes(event.code)) return;
  const mods = [];
  if (event.ctrlKey) mods.push("Ctrl");
  if (event.altKey) mods.push("Alt");
  if (event.shiftKey) mods.push("Shift");
  if (event.metaKey) mods.push("Super");
  const key = readableKey(event.code);
  if (!mods.length && !SOLO_KEYS.test(event.code)) {
    toast("Add Ctrl, Alt or Shift, otherwise that key would stop typing normally", true);
    return;
  }
  const hotkey = [...mods, key].join("+");
  stopRecording();
  updateSettings({ hotkey }, `Hotkey set to ${hotkey}`);
}

function startRecording() {
  const input = $("hotkey");
  input.classList.add("recording");
  input.value = "Press keys… (Esc to cancel)";
  window.addEventListener("keydown", recordHotkey, true);
}

function stopRecording() {
  $("hotkey").classList.remove("recording");
  window.removeEventListener("keydown", recordHotkey, true);
  $("hotkey").blur();
  renderSettings();
}

// ---------- Wiring ----------

function showView(name) {
  document.querySelectorAll(".tab").forEach((t) => t.classList.toggle("active", t.dataset.view === name));
  document.querySelectorAll(".view").forEach((v) => v.classList.toggle("active", v.id === `view-${name}`));
}

document.querySelectorAll(".tab").forEach((tab) =>
  tab.addEventListener("click", () => {
    showView(tab.dataset.view);
  })
);

$("list").addEventListener("click", (e) => {
  const head = e.target.closest(".group-head");
  if (head) {
    toggleGroup(head.dataset.group);
    return;
  }
  const item = e.target.closest(".item");
  if (item) select(item.dataset.id);
});
$("search").addEventListener("input", renderList);
$("new-btn").addEventListener("click", () => select(""));
$("form").addEventListener("submit", save);
$("cancel-btn").addEventListener("click", closeEditor);
$("delete-btn").addEventListener("click", remove);
$("f-content").addEventListener("input", updatePreview);
$("f-rich-editor").addEventListener("input", updatePreview);
// Pasted formatting (from a web page, Word...) is cleaned down to what snippets support.
$("f-rich-editor").addEventListener("paste", (e) => {
  e.preventDefault();
  const html = e.clipboardData.getData("text/html");
  document.execCommand("insertHTML", false, html ? sanitizeHtml(html) : plainToHtml(e.clipboardData.getData("text/plain")));
});
$("f-rich").addEventListener("change", (e) => {
  const editor = $("f-rich-editor");
  if (e.target.checked) {
    editor.innerHTML = plainToHtml($("f-content").value);
    setRich(true);
    editor.focus();
  } else {
    if (editor.querySelector("b, strong, i, em, u, a, ul, ol") && !confirm("Turn off formatting? Bold, italic, links and lists will be removed.")) {
      e.target.checked = true;
      return;
    }
    $("f-content").value = editor.innerText.replace(/\n$/, "");
    setRich(false);
  }
  updatePreview();
});
$("f-sensitive").addEventListener("change", (e) => {
  $("f-rich").disabled = e.target.checked;
});
document.querySelectorAll("#f-rich-toolbar [data-cmd]").forEach((button) =>
  // mousedown + preventDefault keeps the text selection in the editor.
  button.addEventListener("mousedown", (e) => {
    e.preventDefault();
    const command = button.dataset.cmd;
    if (command === "createLink") {
      const url = (prompt("Link address", "https://") || "").trim();
      if (!/^(https?:\/\/\S+|mailto:\S+)$/i.test(url)) return;
      document.execCommand("createLink", false, url);
    } else {
      document.execCommand(command, false);
    }
    updatePreview();
  })
);
// Only the placeholder chips insert text; the formatting toolbar buttons share their look.
document.querySelectorAll(".chip[data-ph]").forEach((chip) => chip.addEventListener("click", () => insertPlaceholder(chip.dataset.ph)));

$("status").addEventListener("click", async () => {
  await invoke("set_paused", { paused: !state.paused });
  await load();
});

$("hotkey").addEventListener("click", startRecording);
$("hotkey").addEventListener("blur", () => $("hotkey").classList.contains("recording") && stopRecording());
$("hotkey-reset").addEventListener("click", () => updateSettings({ hotkey: DEFAULT_HOTKEY }, `Hotkey reset to ${DEFAULT_HOTKEY}`));

$("inline").addEventListener("change", (e) =>
  updateSettings({ inlineEnabled: e.target.checked }, e.target.checked ? "Inline expansion on" : "Inline expansion off")
);
$("excluded-save").addEventListener("click", () =>
  updateSettings({ excludedApps: $("excluded").value.split(/\r?\n/).map((s) => s.trim()).filter(Boolean) }, "Excluded apps saved")
);

$("autostart").addEventListener("change", async (e) => {
  try {
    await invoke("set_autostart", { enabled: e.target.checked });
    toast(e.target.checked ? "Zap will start with Windows" : "Zap won't start with Windows");
  } catch (err) {
    toast(err, true);
  }
  await load();
});

$("check-updates").addEventListener("change", (e) =>
  updateSettings({ checkUpdates: e.target.checked }, e.target.checked ? "Automatic update checks on" : "Automatic update checks off")
);

$("auto-update").addEventListener("change", (e) =>
  updateSettings({ autoUpdate: e.target.checked }, e.target.checked ? "Updates will install automatically" : "Zap will ask before installing updates")
);

$("check-now").addEventListener("click", async () => {
  const button = $("check-now");
  button.disabled = true;
  try {
    const version = await invoke("check_for_updates");
    toast(version ? `Zap ${version} is available` : "You're on the latest version");
  } catch (e) {
    toast(`Couldn't check for updates: ${e}`, true);
  }
  button.disabled = false;
});

$("restart-update").addEventListener("click", async () => {
  const button = $("restart-update");
  restarting = true;
  button.disabled = true;
  button.textContent = "Restarting…";
  try {
    await invoke("install_update"); // Zap restarts when this succeeds
  } catch (e) {
    restarting = false;
    toast(`Couldn't install the update: ${e}`, true);
    button.disabled = false;
    button.textContent = "⟳ Restart to update";
  }
});

$("update-install").addEventListener("click", async () => {
  const button = $("update-install");
  button.disabled = true;
  button.textContent = "Installing…";
  try {
    await invoke("install_update"); // Zap restarts when this succeeds
  } catch (e) {
    toast(`Couldn't install the update: ${e}`, true);
    button.disabled = false;
    button.textContent = "Install and restart";
  }
});

$("f-trigger").addEventListener("input", renderSecondary);
$("secondary-prefix").addEventListener("input", renderSecondary);
$("secondary-save").addEventListener("click", () => {
  const prefix = $("secondary-prefix").value.trim();
  updateSettings({ secondaryPrefix: prefix }, prefix ? `Secondary prefix set to ${prefix}` : "Secondary prefix turned off");
});

$("usage-ping").addEventListener("change", (e) =>
  updateSettings({ usagePing: e.target.checked }, e.target.checked ? "Anonymous user count on" : "Anonymous user count off. The ID is deleted.")
);

$("keep-stats").addEventListener("change", (e) =>
  updateSettings({ keepStats: e.target.checked }, e.target.checked ? "Usage stats on" : "Usage stats off. Nothing will be counted.")
);

$("wpm").addEventListener("change", (e) => {
  const wpm = Math.round(Number(e.target.value));
  if (!wpm || wpm < 5 || wpm > 300) {
    toast("Typing speed must be between 5 and 300 words per minute", true);
    renderStats();
    return;
  }
  updateSettings({ typingWpm: wpm }, `Typing speed set to ${wpm} wpm`);
});

$("stats-reset").addEventListener("click", async () => {
  if (!confirm("Reset all usage stats? Your snippets stay as they are.")) return;
  try {
    await invoke("reset_stats");
    await load();
    toast("Stats reset");
  } catch (e) {
    toast(e, true);
  }
});

$("export-btn").addEventListener("click", async () => {
  try {
    const count = await invoke("export_snippets");
    if (count !== null) toast(`Exported ${count} snippet(s)`);
  } catch (e) {
    toast(e, true);
  }
});

$("import-btn").addEventListener("click", async () => {
  try {
    const result = await invoke("import_snippets");
    if (!result) return;
    await load();
    const skipped = result.skipped.length ? `, skipped ${result.skipped.length}: ${result.skipped.join("; ")}` : "";
    toast(`Imported ${result.imported}${skipped}`, result.skipped.length > 0);
  } catch (e) {
    toast(e, true);
  }
});

document.addEventListener("keydown", (e) => {
  if (e.ctrlKey && e.key.toLowerCase() === "n") {
    e.preventDefault();
    select("");
  } else if (e.ctrlKey && e.key.toLowerCase() === "s" && !$("form").hidden) {
    e.preventDefault();
    $("form").requestSubmit();
  } else if (e.ctrlKey && e.key.toLowerCase() === "f") {
    e.preventDefault();
    $("search").focus();
  }
});

// No browser-style context menu anywhere in the window.
document.addEventListener("contextmenu", (e) => e.preventDefault());

listen("data-changed", load);
listen("main-reopened", () => showView("snippets"));
load();
