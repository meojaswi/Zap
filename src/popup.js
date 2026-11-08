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
  $("add-content").value = "";
  $("add-error").hidden = true;
  showForm(true);
  (words ? $("add-content") : $("add-name")).focus();
}

function closeForm() {
  showForm(false);
  input.focus();
}

async function saveNew() {
  try {
    const saved = await invoke("save_snippet", {
      snippet: { id: "", name: $("add-name").value, trigger: $("add-trigger").value, content: $("add-content").value, sensitive: false, group: "", html: null },
    });
    snippets = (await invoke("get_data")).snippets;
    closeForm();
    // Show the new snippet selected, so Enter inserts it straight away.
    input.value = saved.trigger;
    search();
  } catch (e) {
    $("add-error").textContent = e;
    $("add-error").hidden = false;
  }
}

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
    (e.target.id === "add-name" ? $("add-trigger") : $("add-content")).focus();
  }
});

document.addEventListener("keydown", (e) => {
  if (e.ctrlKey && e.key.toLowerCase() === "n" && $("add-form").hidden) {
    e.preventDefault();
    openForm(results.length ? "" : input.value);
  }
});

async function refresh() {
  showForm(false);
  snippets = (await invoke("get_data")).snippets;
  input.value = "";
  search();
  input.focus();
}

// No browser-style context menu anywhere in the window.
document.addEventListener("contextmenu", (e) => e.preventDefault());

listen("popup-shown", refresh);
listen("data-changed", async () => {
  snippets = (await invoke("get_data")).snippets;
  search();
});
refresh();
