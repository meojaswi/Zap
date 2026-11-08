const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const input = document.getElementById("q");
const resultsEl = document.getElementById("results");

let snippets = [];
let results = [];
let active = 0;

function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML.replace(/"/g, "&quot;");
}

// Exact trigger first, then trigger prefix, then name, then group, then content.
function score(snippet, query) {
  const trigger = snippet.trigger.toLowerCase();
  const bare = trigger.replace(/^`/, "");
  const name = snippet.name.toLowerCase();
  if (trigger === query || bare === query) return 100;
  if (trigger.startsWith(query) || bare.startsWith(query)) return 80;
  if (name.startsWith(query)) return 60;
  if (name.includes(query) || trigger.includes(query)) return 40;
  if ((snippet.group || "").toLowerCase().includes(query)) return 30;
  if (!snippet.sensitive && snippet.content.toLowerCase().includes(query)) return 20;
  return 0;
}

function search() {
  const query = input.value.trim().toLowerCase();
  results = query
    ? snippets
        .map((s) => ({ s, score: score(s, query) }))
        .filter((r) => r.score > 0)
        .sort((a, b) => b.score - a.score || a.s.name.localeCompare(b.s.name))
        .map((r) => r.s)
    : [...snippets].sort((a, b) => a.name.localeCompare(b.name));
  active = 0;
  render();
}

function render() {
  if (!results.length) {
    const query = input.value.trim();
    resultsEl.innerHTML = query
      ? `<div class="empty">No matching snippets</div><button class="add-row" id="add-row">+ Add "${escapeHtml(query)}" as a new snippet</button>`
      : `<div class="empty">No snippets yet. Press + new to add one.</div>`;
    return;
  }
  resultsEl.innerHTML = results
    .map(
      (s, i) => `
      <div class="result ${i === active ? "active" : ""}" data-index="${i}">
        <span class="result-trigger">${escapeHtml(s.trigger)}</span>
        <div class="result-main">
          <div class="result-name">${escapeHtml(s.name.toLowerCase())}${s.group ? `<span class="result-group"> · ${escapeHtml(s.group.toLowerCase())}</span>` : ""}</div>
          <div class="result-preview">${s.sensitive ? "[locked]" : escapeHtml(s.content.replace(/\s+/g, " "))}</div>
        </div>
      </div>`
    )
    .join("");
  resultsEl.querySelector(".result.active")?.scrollIntoView({ block: "nearest" });
}

function choose(index) {
  const snippet = results[index];
  if (snippet) invoke("insert_snippet", { id: snippet.id });
}

input.addEventListener("input", search);

input.addEventListener("keydown", (e) => {
  if (e.key === "ArrowDown") {
    e.preventDefault();
    active = Math.min(active + 1, results.length - 1);
    render();
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    active = Math.max(active - 1, 0);
    render();
  } else if (e.key === "Enter") {
    e.preventDefault();
    if (results.length) choose(active);
    else if (input.value.trim()) openForm(input.value);
  } else if (e.key === "Escape") {
    e.preventDefault();
    invoke("hide_popup");
  }
});

resultsEl.addEventListener("mousemove", (e) => {
  const row = e.target.closest(".result");
  if (row && Number(row.dataset.index) !== active) {
    active = Number(row.dataset.index);
    render();
  }
});
resultsEl.addEventListener("click", (e) => {
  if (e.target.closest("#add-row")) {
    openForm(input.value);
    return;
  }
  const row = e.target.closest(".result");
  if (row) choose(Number(row.dataset.index));
});

// ---------- New snippet ----------

const $ = (id) => document.getElementById(id);
let secondaryPrefix = "";

function renderTriggerHint() {
  const advice = triggerAdvice($("add-trigger").value.trim(), secondaryPrefix);
  $("add-trigger-hint").hidden = !advice;
  $("add-trigger-hint").innerHTML = advice;
}

function showForm(open) {
  $("add-form").hidden = !open;
  $("add-foot").hidden = !open;
  $("search-bar").hidden = open;
  $("search-foot").hidden = open;
  resultsEl.hidden = open;
}

/** Opens the form, using what was searched for as the name and trigger. */
function openForm(query = "") {
  const words = query.trim();
  $("add-name").value = words;
  $("add-trigger").value = "`" + words.toLowerCase().replace(/\s+/g, "");
  renderTriggerHint();
  $("add-content").value = "";
  $("add-rich-editor").innerHTML = "";
  richBackup = null;
  setRich(false);
  generator.reset();
  $("add-link-row").hidden = true;
  $("add-error").hidden = true;
  showForm(true);
  (words ? $("add-content") : $("add-name")).focus();
}

// The form's formatting. No confirm() or prompt() here: a dialog takes focus and the popup hides when it loses focus.
let richBackup = null;
let linkRange = null;

function isRich() {
  return $("add-rich").checked;
}

function setRich(on) {
  $("add-rich").checked = on;
  $("add-content").hidden = on;
  $("add-rich-editor").hidden = !on;
  $("add-rich-toolbar").hidden = !on;
  if (!on) $("add-link-row").hidden = true;
}

function richText() {
  return $("add-rich-editor").innerText.replace(/\n$/, "");
}

function textField() {
  return isRich() ? $("add-rich-editor") : $("add-content");
}

function showFormError(message) {
  $("add-error").textContent = message;
  $("add-error").hidden = false;
}

function openLinkRow() {
  const selection = window.getSelection();
  const inEditor = selection.rangeCount && $("add-rich-editor").contains(selection.anchorNode);
  linkRange = inEditor ? selection.getRangeAt(0).cloneRange() : null;
  $("add-link").value = "https://";
  $("add-link-row").hidden = false;
  $("add-link").focus();
}

function closeLinkRow() {
  $("add-link-row").hidden = true;
  $("add-rich-editor").focus();
  if (linkRange) {
    const selection = window.getSelection();
    selection.removeAllRanges();
    selection.addRange(linkRange);
  }
}

function addLink() {
  const url = $("add-link").value.trim();
  if (!/^(https?:\/\/\S+|mailto:\S+)$/i.test(url)) {
    showFormError("Use a full address starting with https:// or mailto:");
    return;
  }
  $("add-error").hidden = true;
  closeLinkRow();
  if (linkRange && !linkRange.collapsed) {
    document.execCommand("createLink", false, url);
  } else {
    // Nothing selected: the address itself becomes the link.
    const link = document.createElement("a");
    link.href = url;
    link.textContent = url;
    document.execCommand("insertHTML", false, link.outerHTML);
  }
}

function closeForm() {
  showForm(false);
  input.focus();
}

async function saveNew() {
  try {
    const saved = await invoke("save_snippet", {
      snippet: { id: "", name: $("add-name").value, trigger: $("add-trigger").value, content: isRich() ? richText() : $("add-content").value, sensitive: false, group: "", html: isRich() ? sanitizeHtml($("add-rich-editor").innerHTML) : null },
    });
    await reloadData();
    closeForm();
    // Show the new snippet selected, so Enter inserts it straight away.
    input.value = saved.trigger;
    search();
  } catch (e) {
    showFormError(e);
  }
}

$("add-rich").addEventListener("change", (e) => {
  const editor = $("add-rich-editor");
  if (e.target.checked) {
    // Turning it back on with the text untouched brings the formatting back.
    const restore = richBackup && richBackup.text === $("add-content").value;
    editor.innerHTML = restore ? richBackup.html : plainToHtml($("add-content").value);
    setRich(true);
  } else {
    richBackup = { html: editor.innerHTML, text: richText() };
    $("add-content").value = richBackup.text;
    setRich(false);
  }
  textField().focus();
});

document.querySelectorAll("#add-rich-toolbar [data-cmd]").forEach((button) =>
  // mousedown + preventDefault keeps the text selection in the editor.
  button.addEventListener("mousedown", (e) => {
    e.preventDefault();
    if (button.dataset.cmd === "createLink") openLinkRow();
    else document.execCommand(button.dataset.cmd, false);
  })
);

// Pasted formatting (from a web page, Word...) is cleaned down to what snippets support.
$("add-rich-editor").addEventListener("paste", (e) => {
  e.preventDefault();
  const html = e.clipboardData.getData("text/html");
  document.execCommand("insertHTML", false, html ? sanitizeHtml(html) : plainToHtml(e.clipboardData.getData("text/plain")));
});

$("add-link-ok").addEventListener("click", addLink);
$("add-link").addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    e.preventDefault();
    e.stopPropagation();
    addLink();
  } else if (e.key === "Escape") {
    e.preventDefault();
    e.stopPropagation();
    $("add-error").hidden = true;
    closeLinkRow();
  }
});

$("add-trigger").addEventListener("input", renderTriggerHint);
const generator = wireGenerate({
  button: $("add-generate"),
  undo: $("add-generate-undo"),
  fields: [$("add-content"), $("add-rich-editor")],
  getText: () => (isRich() ? richText() : $("add-content").value),
  setText(text) {
    if (isRich()) $("add-rich-editor").innerHTML = plainToHtml(text);
    else $("add-content").value = text;
  },
  blocked: () => "",
  showError: showFormError,
});

$("new-btn").addEventListener("click", () => openForm());
$("add-save").addEventListener("click", saveNew);
$("add-cancel").addEventListener("click", closeForm);

$("add-form").addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    e.preventDefault();
    closeForm();
  } else if (e.key === "Enter" && e.ctrlKey) {
    e.preventDefault();
    saveNew();
  } else if (e.key === "Enter" && e.target.tagName === "INPUT") {
    // Enter moves on from Name and Trigger; in Text it's a new line.
    e.preventDefault();
    (e.target.id === "add-name" ? $("add-trigger") : textField()).focus();
  }
});

document.addEventListener("keydown", (e) => {
  if (e.ctrlKey && e.key.toLowerCase() === "n" && $("add-form").hidden) {
    e.preventDefault();
    openForm(results.length ? "" : input.value);
  }
});

async function reloadData() {
  const data = await invoke("get_data");
  snippets = data.snippets;
  secondaryPrefix = data.settings.secondaryPrefix;
}

async function refresh() {
  showForm(false);
  await reloadData();
  input.value = "";
  search();
  input.focus();
}

// No browser-style context menu anywhere in the window.
document.addEventListener("contextmenu", (e) => e.preventDefault());

listen("popup-shown", refresh);
listen("popup-new-from-selection", async (e) => {
  await reloadData();
  input.value = "";
  search();
  openForm();
  if (e.payload.text) $("add-content").value = e.payload.text;
  else showFormError(e.payload.error);
});
listen("data-changed", async () => {
  await reloadData();
  search();
});
refresh();
