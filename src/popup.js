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

// Exact trigger first, then trigger prefix, then name, then content.
function score(snippet, query) {
  const trigger = snippet.trigger.toLowerCase();
  const bare = trigger.replace(/^`/, "");
  const name = snippet.name.toLowerCase();
  if (trigger === query || bare === query) return 100;
  if (trigger.startsWith(query) || bare.startsWith(query)) return 80;
  if (name.startsWith(query)) return 60;
  if (name.includes(query) || trigger.includes(query)) return 40;
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
    resultsEl.innerHTML = `<div class="empty">${snippets.length ? "No matching snippets" : "No snippets yet. Add some in the Zap window."}</div>`;
    return;
  }
  resultsEl.innerHTML = results
    .map(
      (s, i) => `
      <div class="result ${i === active ? "active" : ""}" data-index="${i}">
        <span class="result-trigger">${escapeHtml(s.trigger)}</span>
        <div class="result-main">
          <div class="result-name">${escapeHtml(s.name.toLowerCase())}</div>
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
    choose(active);
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
  const row = e.target.closest(".result");
  if (row) choose(Number(row.dataset.index));
});

async function refresh() {
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
