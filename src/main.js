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

function renderStatus() {
  $("status").classList.toggle("paused", state.paused);
  $("status-text").textContent = state.paused ? "Paused" : "Active";
  $("welcome-hotkey").textContent = state.settings.hotkey;
  const banner = $("hotkey-banner");
  banner.hidden = !state.hotkeyError;
  banner.textContent = state.hotkeyError
    ? `${state.hotkeyError}. Another app may be using it, so pick a different one in Settings.`
    : "";
  $("brand-version").textContent = state.appVersion;
  $("update-banner").hidden = !state.updateVersion;
  $("update-text").textContent = `Zap ${state.updateVersion} is available.`;
}

function renderList() {
  const query = $("search").value.trim().toLowerCase();
  const items = state.snippets
    .filter((s) => {
      if (!query) return true;
      const haystack = [s.name, s.trigger, s.sensitive ? "" : s.content].join(" ").toLowerCase();
      return haystack.includes(query);
    })
    .sort((a, b) => a.name.localeCompare(b.name));

  const list = $("list");
  if (!items.length) {
    list.innerHTML = `<div class="empty">${state.snippets.length ? "No matches" : "No snippets yet"}</div>`;
    return;
  }
  list.innerHTML = items
    .map(
      (s) => `
      <button class="item ${s.id === selectedId ? "active" : ""}" data-id="${s.id}" title="${escapeHtml(s.name)}">
        <span class="item-trigger">${escapeHtml(s.trigger)}</span>
        <span class="item-name">${escapeHtml(s.name.toLowerCase())}${s.sensitive ? ' <span class="lock" title="Sensitive">[locked]</span>' : ""}</span>
      </button>`
    )
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
  $("delete-btn").hidden = creating;
  renderUsage();
  updatePreview();
  renderList();
  (creating ? $("f-name") : $("f-content")).focus();
}

function closeEditor() {
  selectedId = null;
  $("form").hidden = true;
  $("welcome").hidden = false;
  renderList();
  renderUsage();
}

let previewTimer;
function updatePreview() {
  clearTimeout(previewTimer);
  previewTimer = setTimeout(async () => {
    const content = $("f-content").value;
    $("preview-field").hidden = !/\{(date|time|datetime|day|clipboard|uuid)(:[^}]*)?\}/.test(content);
    if (!$("preview-field").hidden) $("preview").textContent = await invoke("preview", { content });
  }, 150);
}

function insertPlaceholder(text) {
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
        content: $("f-content").value,
        sensitive: $("f-sensitive").checked,
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

// ---------- Settings ----------

function renderSettings() {
  if (!$("hotkey").classList.contains("recording")) $("hotkey").value = state.settings.hotkey;
  $("inline").checked = state.settings.inlineEnabled;
  $("autostart").checked = state.autostart;
  $("check-updates").checked = state.settings.checkUpdates;
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
  const item = e.target.closest(".item");
  if (item) select(item.dataset.id);
});
$("search").addEventListener("input", renderList);
$("new-btn").addEventListener("click", () => select(""));
$("form").addEventListener("submit", save);
$("cancel-btn").addEventListener("click", closeEditor);
$("delete-btn").addEventListener("click", remove);
$("f-content").addEventListener("input", updatePreview);
document.querySelectorAll(".chip").forEach((chip) => chip.addEventListener("click", () => insertPlaceholder(chip.dataset.ph)));

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
