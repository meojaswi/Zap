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
  updatePreview();
  renderList();
  (creating ? $("f-name") : $("f-content")).focus();
}

function closeEditor() {
  selectedId = null;
  $("form").hidden = true;
  $("welcome").hidden = false;
  renderList();
}

let previewTimer;
function updatePreview() {
  clearTimeout(previewTimer);
  previewTimer = setTimeout(async () => {
    const content = $("f-content").value;
    $("preview-field").hidden = !/\{(date|time|datetime|day|clipboard|uuid)\}/.test(content);
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

document.querySelectorAll(".tab").forEach((tab) =>
  tab.addEventListener("click", () => {
    document.querySelectorAll(".tab").forEach((t) => t.classList.toggle("active", t === tab));
    document.querySelectorAll(".view").forEach((v) => v.classList.toggle("active", v.id === `view-${tab.dataset.view}`));
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

listen("data-changed", load);
load();
